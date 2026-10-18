use faktura_application::company::{CompanyError as Error, Result};
use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
#[derive(Clone)]
pub struct MailboxConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mailbox: String,
}
pub struct Message {
    pub uid: u32,
    pub body: Option<Vec<u8>>,
}
pub struct Batch {
    pub uid_validity: u32,
    pub messages: Vec<Message>,
}
pub trait Transport: Send + Sync {
    fn fetch(&self, config: MailboxConfig, validity: Option<u32>, cursor: u32) -> Result<Batch>;
}
pub struct ImapTls;
fn bad() -> Error {
    Error::Unavailable
}
struct DeadlineTcp {
    inner: TcpStream,
    deadline: std::time::Instant,
}
impl DeadlineTcp {
    fn remaining(&self) -> std::io::Result<Duration> {
        let remaining = self
            .deadline
            .saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Mailbox deadline",
            ));
        }
        Ok(remaining.min(Duration::from_secs(20)))
    }
}
impl Read for DeadlineTcp {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        self.inner.set_read_timeout(Some(self.remaining()?))?;
        self.inner.read(out)
    }
}
impl Write for DeadlineTcp {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.inner.set_write_timeout(Some(self.remaining()?))?;
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
struct Budget<T> {
    inner: T,
    remaining: Arc<AtomicUsize>,
    deadline: std::time::Instant,
}
impl<T: Read> Read for Budget<T> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if std::time::Instant::now() >= self.deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Mailbox deadline",
            ));
        }
        let max = self.remaining.load(Ordering::Relaxed).min(out.len());
        if max == 0 {
            return Err(std::io::Error::other("Mailbox response limit"));
        }
        let n = self.inner.read(&mut out[..max])?;
        self.remaining.fetch_sub(n, Ordering::Relaxed);
        Ok(n)
    }
}
impl<T: Write> Write for Budget<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
impl Transport for ImapTls {
    fn fetch(&self, config: MailboxConfig, validity: Option<u32>, cursor: u32) -> Result<Batch> {
        let addresses = (config.host.as_str(), config.port)
            .to_socket_addrs()
            .map_err(|_| bad())?;
        let mut socket = None;
        for address in addresses.take(4) {
            if let Ok(s) = TcpStream::connect_timeout(&address, Duration::from_secs(10)) {
                socket = Some(s);
                break;
            }
        }
        let socket = socket.ok_or_else(bad)?;
        socket
            .set_read_timeout(Some(Duration::from_secs(20)))
            .map_err(|_| bad())?;
        socket
            .set_write_timeout(Some(Duration::from_secs(20)))
            .map_err(|_| bad())?;
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| bad())?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(config.host).map_err(|_| bad())?;
        let connection = rustls::ClientConnection::new(Arc::new(tls), name).map_err(|_| bad())?;
        let remaining = Arc::new(AtomicUsize::new(16384));
        let stream = Budget {
            inner: rustls::StreamOwned::new(
                connection,
                DeadlineTcp {
                    inner: socket,
                    deadline: std::time::Instant::now() + Duration::from_secs(120),
                },
            ),
            remaining: remaining.clone(),
            deadline: std::time::Instant::now() + Duration::from_secs(120),
        };
        fetch_stream(
            stream,
            remaining,
            &config.username,
            &config.password,
            &config.mailbox,
            validity,
            cursor,
        )
    }
}
fn fetch_stream<T: Read + Write>(
    stream: T,
    budget: Arc<AtomicUsize>,
    username: &str,
    password: &str,
    mailbox: &str,
    validity: Option<u32>,
    cursor: u32,
) -> Result<Batch> {
    let mut client = imap::Client::new(stream);
    client.read_greeting().map_err(|_| bad())?;
    budget.store(16384, Ordering::Relaxed);
    let mut session = client.login(username, password).map_err(|_| bad())?;
    budget.store(65536, Ordering::Relaxed);
    let selected = session.examine(mailbox).map_err(|_| bad())?;
    let current = selected.uid_validity.filter(|v| *v > 0).ok_or_else(bad)?;
    let cursor = if validity == Some(current) { cursor } else { 0 };
    if cursor == u32::MAX {
        let _ = session.logout();
        return Ok(Batch {
            uid_validity: current,
            messages: Vec::new(),
        });
    }
    budget.store(512 * 1024, Ordering::Relaxed);
    let mut uids = session
        .uid_search(format!("UID {}:* NOT DELETED", cursor + 1))
        .map_err(|_| bad())?
        .into_iter()
        .filter(|uid| *uid > cursor)
        .collect::<Vec<_>>();
    uids.sort_unstable();
    uids.truncate(10);
    let mut messages = Vec::new();
    let mut total = 0;
    for uid in uids {
        budget.store(16384, Ordering::Relaxed);
        let sizes = session
            .uid_fetch(uid.to_string(), "(UID RFC822.SIZE)")
            .map_err(|_| bad())?;
        let row = sizes.iter().find(|r| r.uid == Some(uid)).ok_or_else(bad)?;
        let size = row.size.ok_or_else(bad)? as usize;
        if size > 10 * 1024 * 1024 {
            messages.push(Message { uid, body: None });
            continue;
        }
        if total + size > 20 * 1024 * 1024 {
            break;
        }
        budget.store(11 * 1024 * 1024, Ordering::Relaxed);
        let fetch = session
            .uid_fetch(uid.to_string(), "(UID BODY.PEEK[])")
            .map_err(|_| bad())?;
        let row = fetch.iter().find(|r| r.uid == Some(uid)).ok_or_else(bad)?;
        let body = row.body().ok_or_else(bad)?;
        if body.len() > 10 * 1024 * 1024 || body.len() != size {
            return Err(bad());
        }
        total += body.len();
        messages.push(Message {
            uid,
            body: Some(body.to_vec()),
        });
    }
    budget.store(16384, Ordering::Relaxed);
    let _ = session.logout();
    Ok(Batch {
        uid_validity: current,
        messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_transport_rejects_plaintext_instead_of_downgrading_tls() {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = [0; 4096];
            let n = socket.read(&mut bytes).unwrap();
            assert!(n > 0);
            assert_eq!(bytes[0], 22);
            assert!(!String::from_utf8_lossy(&bytes[..n]).contains("secret-password"));
            socket.write_all(b"* OK plaintext is not TLS\r\n").unwrap();
        });
        assert!(ImapTls
            .fetch(
                MailboxConfig {
                    host: "127.0.0.1".into(),
                    port,
                    username: "synthetic".into(),
                    password: "secret-password".into(),
                    mailbox: "INBOX".into()
                },
                None,
                0
            )
            .is_err());
        thread.join().unwrap();
    }
    #[test]
    fn loopback_imap_uid_reset_bounded_fetch_and_peek() {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = server.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let (socket, _) = server.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(socket);
            reader
                .get_mut()
                .write_all(b"* OK synthetic loopback\r\n")
                .unwrap();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.is_empty() {
                    break;
                }
                let (tag, command) = line.trim_end().split_once(' ').unwrap();
                let response = if command.starts_with("LOGIN ") {
                    assert!(command.contains("synthetic-user"));
                    format!("{tag} OK login\r\n")
                } else if command.starts_with("EXAMINE ") {
                    format!("* 1 EXISTS\r\n* OK [UIDVALIDITY 9] epoch\r\n{tag} OK selected\r\n")
                } else if command.starts_with("UID SEARCH ") {
                    assert!(command.contains("UID 1:*"));
                    format!("* SEARCH 1\r\n{tag} OK search\r\n")
                } else if command.contains("RFC822.SIZE") {
                    format!("* 1 FETCH (UID 1 RFC822.SIZE 4)\r\n{tag} OK fetch\r\n")
                } else if command.contains("BODY.PEEK[]") {
                    format!("* 1 FETCH (UID 1 BODY[] {{4}}\r\nTEST)\r\n{tag} OK fetch\r\n")
                } else if command == "LOGOUT" {
                    reader
                        .get_mut()
                        .write_all(format!("* BYE\r\n{tag} OK logout\r\n").as_bytes())
                        .unwrap();
                    break;
                } else {
                    panic!("unexpected synthetic IMAP command")
                };
                reader.get_mut().write_all(response.as_bytes()).unwrap();
            }
        });
        let socket = TcpStream::connect(address).unwrap();
        let budget = Arc::new(AtomicUsize::new(16384));
        let stream = Budget {
            inner: socket,
            remaining: budget.clone(),
            deadline: std::time::Instant::now() + Duration::from_secs(10),
        };
        let batch = fetch_stream(
            stream,
            budget,
            "synthetic-user",
            "synthetic-password",
            "INBOX",
            Some(8),
            100,
        )
        .unwrap();
        assert_eq!(batch.uid_validity, 9);
        assert_eq!(batch.messages[0].uid, 1);
        assert_eq!(batch.messages[0].body.as_deref(), Some(b"TEST".as_slice()));
        thread.join().unwrap();
    }
}

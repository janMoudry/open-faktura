//! Only installation-approved endpoints receive bank tokens. Never expose URLs/errors.
use async_trait::async_trait;
use faktura_application::bank_connection::{Error, Provider, Result};
use serde_json::Value;
use std::time::Duration;
#[derive(Clone)]
pub struct Endpoint {
    pub provider: Provider,
    pub url: String,
    pub format: String,
}
pub fn endpoints() -> Result<Vec<Endpoint>> {
    let mut list = vec![Endpoint {
        provider: Provider {
            id: "fio".into(),
            label: "Fio API Banking".into(),
        },
        url: "https://fioapi.fio.cz/v1/rest".into(),
        format: "fio_json".into(),
    }];
    if let Ok(raw) = std::env::var("BANK_GATEWAYS_JSON") {
        let values: Vec<Value> = serde_json::from_str(&raw).map_err(|_| Error::Unavailable)?;
        for v in values {
            let field = |k| v[k].as_str().map(str::to_owned).ok_or(Error::Unavailable);
            let id = field("id")?;
            let label = field("label")?;
            let url = field("url")?;
            let format = field("format")?;
            let u = reqwest::Url::parse(&url).map_err(|_| Error::Unavailable)?;
            if id.is_empty()
                || id.len() > 64
                || !id
                    .bytes()
                    .all(|v| v.is_ascii_alphanumeric() || v == b'_' || v == b'-')
                || list.iter().any(|p| p.provider.id == id)
                || label.is_empty()
                || label.len() > 100
                || u.scheme() != "https"
                || !u.username().is_empty()
                || u.password().is_some()
                || u.query().is_some()
                || u.fragment().is_some()
                || !["camt053", "fio_json", "moneta_xml"].contains(&format.as_str())
            {
                return Err(Error::Unavailable);
            }
            list.push(Endpoint {
                provider: Provider { id, label },
                url,
                format,
            });
        }
    }
    Ok(list)
}
#[async_trait]
pub trait Transport: Send + Sync {
    async fn fetch(
        &self,
        endpoint: &Endpoint,
        token: &str,
        iban: &str,
        from: &str,
        to: &str,
    ) -> std::result::Result<Vec<u8>, String>;
}
pub struct HttpTransport;
#[async_trait]
impl Transport for HttpTransport {
    async fn fetch(
        &self,
        ep: &Endpoint,
        token: &str,
        iban: &str,
        from: &str,
        to: &str,
    ) -> std::result::Result<Vec<u8>, String> {
        let fail = || "connection_network".to_owned();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| fail())?;
        let request = if ep.provider.id == "fio" {
            client.get(format!(
                "{}/periods/{token}/{from}/{to}/transactions.json",
                ep.url
            ))
        } else {
            let mut url = reqwest::Url::parse(&ep.url).map_err(|_| fail())?;
            url.query_pairs_mut()
                .extend_pairs([("iban", iban), ("from", from), ("to", to)]);
            client.get(url).bearer_auth(token)
        };
        let mut r = request.send().await.map_err(|_| fail())?;
        if !r.status().is_success() {
            return Err(match r.status().as_u16() {
                401 | 403 => "connection_auth",
                409 | 429 => "connection_rate",
                _ => "connection_network",
            }
            .into());
        }
        let mut bytes = vec![];
        while let Some(chunk) = r.chunk().await.map_err(|_| fail())? {
            if bytes.len() + chunk.len() > 1024 * 1024 {
                return Err("connection_size".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    async fn serve(status: &str, body: &str) -> (std::result::Result<Vec<u8>, String>, String) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let response=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nLocation: https://must-not-follow.example.test/\r\nConnection: close\r\n\r\n{body}",body.len());
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut b = [0; 4096];
            let n = stream.read(&mut b).unwrap();
            let _ = stream.write_all(response.as_bytes());
            String::from_utf8_lossy(&b[..n]).into_owned()
        });
        // Explicit injected loopback endpoint only in tests; installation gateway validation requires HTTPS.
        let ep = Endpoint {
            provider: Provider {
                id: "test".into(),
                label: "Test".into(),
            },
            url: format!("http://{address}/statement"),
            format: "camt053".into(),
        };
        let result = HttpTransport
            .fetch(
                &ep,
                "synthetic-token",
                "CZ6508000000192000145399",
                "2026-10-01",
                "2026-10-14",
            )
            .await;
        (result, thread.join().unwrap())
    }
    #[tokio::test]
    async fn request_auth_query_errors_and_redirect_block() {
        let (result, request) = serve("200 OK", "synthetic payload").await;
        assert_eq!(result.unwrap(), b"synthetic payload");
        assert!(request
            .to_lowercase()
            .contains("authorization: bearer synthetic-token"));
        assert!(request.contains("from=2026-10-01"));
        assert!(request.contains("iban=CZ6508000000192000145399"));
        assert_eq!(
            serve("302 Found", "").await.0.unwrap_err(),
            "connection_network"
        );
        assert_eq!(
            serve("403 Forbidden", "").await.0.unwrap_err(),
            "connection_auth"
        );
        assert_eq!(
            serve("429 Too Many Requests", "").await.0.unwrap_err(),
            "connection_rate"
        );
        assert_eq!(
            serve("200 OK", &"x".repeat(1024 * 1024 + 1))
                .await
                .0
                .unwrap_err(),
            "connection_size"
        );
    }
}

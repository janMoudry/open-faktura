#[path = "inbox_mailbox_mime.rs"]
mod mime;
#[path = "inbox_mailbox_transport.rs"]
mod transport;
use async_trait::async_trait;
use faktura_application::inbox_mailbox::*;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
pub use transport::{Batch, MailboxConfig, Message, Transport};
pub struct PgMailbox {
    pool: PgPool,
    transport: Arc<dyn Transport>,
}
impl PgMailbox {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            transport: Arc::new(transport::ImapTls),
        }
    }
    pub fn with_transport(pool: PgPool, transport: Arc<dyn Transport>) -> Self {
        Self { pool, transport }
    }
}
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn bad() -> Error {
    Error::Invalid(vec!["mailbox".into()])
}
fn config(name: &str) -> Result<MailboxConfig> {
    if name.is_empty() || name.len() > 80 {
        return Err(bad());
    }
    let raw = std::env::var("INBOX_IMAP_PROVIDERS_JSON").map_err(|_| bad())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|_| bad())?;
    let v = v.get(name).ok_or_else(bad)?;
    let string = |key: &str, max: usize| -> Result<String> {
        let value = v.get(key).and_then(|v| v.as_str()).ok_or_else(bad)?;
        if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
            return Err(bad());
        }
        Ok(value.into())
    };
    let host = string("host", 253)?;
    rustls::pki_types::ServerName::try_from(host.clone()).map_err(|_| bad())?;
    let port = v
        .get("port")
        .map(|v| v.as_u64().filter(|p| *p > 0 && *p <= 65535).ok_or_else(bad))
        .transpose()?
        .unwrap_or(993) as u16;
    Ok(MailboxConfig {
        host,
        port,
        username: string("username", 1024)?,
        password: string("password", 1024)?,
        mailbox: string("mailbox", 200)?,
    })
}
fn map(r: &sqlx::postgres::PgRow) -> Settings {
    Settings {
        revision: r.get("revision"),
        enabled: r.get("enabled"),
        automatic: r.get("automatic"),
        next_run_at: r
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("next_run_at")
            .map(|v| v.to_rfc3339()),
        provider: r.get("provider"),
        uid_validity: r.get::<Option<i64>, _>("uid_validity").map(|v| v as u32),
        cursor: r.get::<i64, _>("cursor") as u32,
    }
}
async fn load(tx: &mut Transaction<'_, Postgres>, c: i64) -> Result<Settings> {
    Ok(
        sqlx::query("SELECT * FROM inbox_mailbox_settings WHERE company_id=$1")
            .bind(c)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .as_ref()
            .map(map)
            .unwrap_or(Settings {
                revision: 0,
                enabled: false,
                automatic: false,
                next_run_at: None,
                provider: None,
                uid_validity: None,
                cursor: 0,
            }),
    )
}
async fn authlock(tx: &mut Transaction<'_, Postgres>, a: &str, c: &str) -> Result<(i64, i64)> {
    let ids = crate::draft::authorize(tx, a, c, true).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(ids.1)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok(ids)
}
async fn check(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    request: &str,
    revision: i64,
) -> Result<(i64, i64, Settings)> {
    let (a, c) = authlock(tx, a, c).await?;
    let setting = load(tx, c).await?;
    let held: bool = sqlx::query_scalar(
        "SELECT coalesce(running_request=$2::text::uuid,false) FROM inbox_mailbox_settings WHERE company_id=$1",
    )
    .bind(c)
    .bind(request)
    .fetch_one(&mut **tx)
    .await
    .map_err(err)?;
    if !setting.enabled || setting.revision != revision || !held {
        return Err(Error::Conflict);
    }
    Ok((a, c, setting))
}
#[async_trait]
impl MailboxStore for PgMailbox {
    async fn settings(&self, a: &str, c: &str) -> Result<Settings> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c).await
    }
    async fn configure(&self, a: &str, c: &str, input: Configure) -> Result<Settings> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (a, c) = authlock(&mut tx, a, c).await?;
        let old = load(&mut tx, c).await?;
        if old.revision != input.revision {
            return Err(Error::Conflict);
        }
        if input.automatic && !input.enabled {
            return Err(bad());
        }
        if input.enabled && input.provider.is_none() {
            return Err(bad());
        }
        if let Some(name) = &input.provider {
            config(name)?;
        }
        let changed = old.provider != input.provider;
        sqlx::query(
            "UPDATE inbox_mailbox_run SET status='failed',error='configuration_changed' WHERE company_id=$1 AND status='running'",
        )
        .bind(c)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        sqlx::query("INSERT INTO inbox_mailbox_settings(company_id,actor_id,enabled,provider,automatic,next_run_at) VALUES($1,$2,$3,$4,$6,CASE WHEN $6 THEN now() ELSE NULL END) ON CONFLICT(company_id) DO UPDATE SET actor_id=$2,enabled=$3,provider=$4,automatic=$6,next_run_at=CASE WHEN $6 THEN now() ELSE NULL END,revision=inbox_mailbox_settings.revision+1,uid_validity=CASE WHEN $5 THEN NULL ELSE inbox_mailbox_settings.uid_validity END,cursor=CASE WHEN $5 THEN 0 ELSE inbox_mailbox_settings.cursor END,running_request=NULL,claimed_at=NULL,updated_at=now()")
   .bind(c).bind(a).bind(input.enabled).bind(input.provider).bind(changed).bind(input.automatic).execute(&mut *tx).await.map_err(err)?;
        let result = load(&mut tx, c).await?;
        sqlx::query("INSERT INTO inbox_mailbox_settings_audit(company_id,actor_id,revision,enabled,provider,automatic) VALUES($1,$2,$3,$4,$5,$6)").bind(c).bind(a).bind(result.revision).bind(result.enabled).bind(&result.provider).bind(result.automatic).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn history(&self, a: &str, c: &str) -> Result<Vec<Run>> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let rows=sqlx::query("SELECT * FROM inbox_mailbox_run WHERE company_id=$1 ORDER BY created_at DESC LIMIT 100").bind(c).fetch_all(&mut *tx).await.map_err(err)?;
        rows.into_iter()
            .map(|r| {
                if let Some(v) = r.get::<Option<serde_json::Value>, _>("result") {
                    serde_json::from_value(v).map_err(err)
                } else {
                    Ok(Run {
                        request_id: r.get::<uuid::Uuid, _>("request_id").to_string(),
                        status: r.get("status"),
                        error: if r.get::<String, _>("status") == "failed" {
                            r.get::<Option<String>, _>("error")
                                .or_else(|| Some("interrupted".into()))
                        } else {
                            None
                        },
                        messages: 0,
                        attachments: 0,
                        rejected: 0,
                        uid_validity: None,
                        cursor: 0,
                        created_at: r
                            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                            .to_rfc3339(),
                    })
                }
            })
            .collect()
    }
    async fn sync(&self, actor: &str, company: &str, request: &str) -> Result<Run> {
        self.sync_mode(actor, company, request, None).await
    }
}
impl PgMailbox {
    async fn sync_mode(
        &self,
        actor: &str,
        company: &str,
        request: &str,
        expected: Option<i64>,
    ) -> Result<Run> {
        uuid::Uuid::parse_str(request).map_err(|_| bad())?;
        let lease = uuid::Uuid::new_v4().to_string();
        let result = self
            .sync_inner(actor, company, request, &lease, expected)
            .await;
        if result.is_err() {
            let error = match &result {
                Err(Error::Forbidden) => "forbidden",
                Err(Error::Invalid(_)) => "invalid",
                Err(Error::Conflict) => "conflict",
                _ => "unavailable",
            };
            // Release only this actor's own already-authorized claim. No financial data
            // or provider secrets are persisted by failure cleanup.
            if let (Ok(a), Ok(c)) = (actor.parse::<i64>(), company.parse::<i64>()) {
                let _=sqlx::query("UPDATE inbox_mailbox_run SET status='failed',error=$5 WHERE company_id=$1 AND request_id=$2::text::uuid AND actor_id=$3 AND status='running' AND EXISTS(SELECT 1 FROM inbox_mailbox_settings s WHERE s.company_id=$1 AND s.running_request=$4::text::uuid)").bind(c).bind(request).bind(a).bind(&lease).bind(error).execute(&self.pool).await;
                let _=sqlx::query("UPDATE inbox_mailbox_settings SET running_request=NULL,claimed_at=NULL WHERE company_id=$1 AND running_request=$2::text::uuid").bind(c).bind(&lease).execute(&self.pool).await;
            }
        }
        result
    }
    async fn sync_inner(
        &self,
        actor: &str,
        company: &str,
        request: &str,
        lease: &str,
        expected: Option<i64>,
    ) -> Result<Run> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (a, c) = authlock(&mut tx, actor, company).await?;
        let settings = load(&mut tx, c).await?;
        if expected.is_some_and(|r| r != settings.revision) {
            return Err(Error::Conflict);
        }
        if !settings.enabled {
            return Err(Error::Forbidden);
        }
        let provider = settings.provider.clone().ok_or_else(bad)?;
        let transport_config = config(&provider)?;
        let previous = sqlx::query(
            "SELECT * FROM inbox_mailbox_run WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(request)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
        if let Some(r) = previous {
            if r.get::<i64, _>("actor_id") != a
                || r.get::<i64, _>("settings_revision") != settings.revision
            {
                return Err(Error::Conflict);
            }
            if let Some(value) = r.get::<Option<serde_json::Value>, _>("result") {
                return serde_json::from_value(value).map_err(err);
            }
        }
        let busy:bool=sqlx::query_scalar("SELECT running_request IS NOT NULL AND claimed_at>now()-interval '5 minutes' FROM inbox_mailbox_settings WHERE company_id=$1").bind(c).fetch_one(&mut *tx).await.map_err(err)?;
        if busy {
            return Err(Error::Conflict);
        }
        sqlx::query(
            "UPDATE inbox_mailbox_run SET status='failed' WHERE company_id=$1 AND status='running'",
        )
        .bind(c)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        sqlx::query("INSERT INTO inbox_mailbox_run(company_id,request_id,actor_id,settings_revision,status) VALUES($1,$2::text::uuid,$3,$4,'running') ON CONFLICT(company_id,request_id) DO UPDATE SET status='running',error=NULL").bind(c).bind(request).bind(a).bind(settings.revision).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE inbox_mailbox_settings SET running_request=$2::text::uuid,claimed_at=now() WHERE company_id=$1").bind(c).bind(lease).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        let transport = self.transport.clone();
        let validity = settings.uid_validity;
        let cursor = settings.cursor;
        static LIMIT: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> = std::sync::OnceLock::new();
        let permit = LIMIT
            .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(4)))
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Unavailable)?;
        let batch = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            transport.fetch(transport_config, validity, cursor)
        })
        .await
        .map_err(err)??;
        if batch.uid_validity == 0
            || batch.messages.len() > 10
            || batch.messages.windows(2).any(|w| w[0].uid >= w[1].uid)
            || batch.messages.iter().any(|m| m.uid == 0)
        {
            return Err(bad());
        }
        let mut result = Run {
            request_id: request.into(),
            status: "completed".into(),
            error: None,
            messages: 0,
            attachments: 0,
            rejected: 0,
            uid_validity: Some(batch.uid_validity),
            cursor: if validity == Some(batch.uid_validity) {
                cursor
            } else {
                0
            },
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        for message in batch.messages {
            if message.uid <= result.cursor {
                return Err(bad());
            }
            let mut tx = self.pool.begin().await.map_err(err)?;
            let (a, c, _) = check(&mut tx, actor, company, lease, settings.revision).await?;
            let input = message.body.as_ref().ok_or_else(bad).and_then(|bytes| {
                let mut digest = Sha256::new();
                digest.update(provider.as_bytes());
                digest.update(batch.uid_validity.to_be_bytes());
                digest.update(message.uid.to_be_bytes());
                let hash = digest.finalize();
                let mut bytes_id = [0u8; 16];
                bytes_id.copy_from_slice(&hash[..16]);
                mime::parse(
                    bytes,
                    uuid::Uuid::from_bytes(bytes_id).to_string(),
                    &format!("imap:{provider}:{}:{}", batch.uid_validity, message.uid),
                )
            });
            // A savepoint keeps a rejected message from rolling back already imported
            // messages; its cursor advances only alongside explicit rejection diagnostics.
            sqlx::query("SAVEPOINT mailbox_message")
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            let intake = match input {
                Ok(input) => crate::inbox::mail_in_tx(&mut tx, a, c, input, true).await,
                Err(e) => Err(e),
            };
            let (status, error) = match intake {
                Ok(mail) => {
                    result.attachments += mail.documents.len() as i32;
                    ("imported", None)
                }
                Err(Error::Invalid(_) | Error::Conflict) => {
                    sqlx::query("ROLLBACK TO SAVEPOINT mailbox_message")
                        .execute(&mut *tx)
                        .await
                        .map_err(err)?;
                    result.rejected += 1;
                    ("rejected", Some("unsupported_message"))
                }
                Err(e) => return Err(e),
            };
            sqlx::query("INSERT INTO inbox_mailbox_message(company_id,provider,uid_validity,uid,status,error) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(company_id,provider,uid_validity,uid) DO NOTHING").bind(c).bind(&provider).bind(batch.uid_validity as i64).bind(message.uid as i64).bind(status).bind(error).execute(&mut *tx).await.map_err(err)?;
            sqlx::query(
                "UPDATE inbox_mailbox_settings SET uid_validity=$2,cursor=$3 WHERE company_id=$1",
            )
            .bind(c)
            .bind(batch.uid_validity as i64)
            .bind(message.uid as i64)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
            tx.commit().await.map_err(err)?;
            result.messages += 1;
            result.cursor = message.uid;
        }
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c, _) = check(&mut tx, actor, company, lease, settings.revision).await?;
        sqlx::query("UPDATE inbox_mailbox_settings SET uid_validity=$2,cursor=$3,running_request=NULL,claimed_at=NULL WHERE company_id=$1").bind(c).bind(batch.uid_validity as i64).bind(result.cursor as i64).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE inbox_mailbox_run SET status='completed',result=$3 WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(request).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

impl PgMailbox {
    /// Claim one due company without holding its SQL row during mailbox I/O.
    pub async fn run_scheduled(&self) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let row=sqlx::query("SELECT company_id,actor_id,revision FROM inbox_mailbox_settings WHERE enabled AND automatic AND next_run_at<=now() AND (running_request IS NULL OR claimed_at<now()-interval '5 minutes') ORDER BY next_run_at,company_id FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(err)?;
        let Some(row) = row else { return Ok(false) };
        let c: i64 = row.get("company_id");
        let a: i64 = row.get("actor_id");
        let revision: i64 = row.get("revision");
        sqlx::query("UPDATE inbox_mailbox_settings SET next_run_at=now()+interval '5 minutes' WHERE company_id=$1").bind(c).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        let request = uuid::Uuid::new_v4().to_string();
        if let Err(error) = self
            .sync_mode(&a.to_string(), &c.to_string(), &request, Some(revision))
            .await
        {
            let error = match error {
                Error::Forbidden => "forbidden",
                Error::Conflict => "conflict",
                Error::Invalid(_) => "invalid",
                _ => "unavailable",
            };
            sqlx::query("INSERT INTO inbox_mailbox_run(company_id,request_id,actor_id,settings_revision,status,error) VALUES($1,$2::text::uuid,$3,$4,'failed',$5) ON CONFLICT(company_id,request_id) DO NOTHING").bind(c).bind(request).bind(a).bind(revision).bind(error).execute(&self.pool).await.map_err(err)?;
        }
        Ok(true)
    }
}

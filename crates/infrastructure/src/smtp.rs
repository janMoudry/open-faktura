//! Installation SMTP settings. Secret material never enters public DTOs or audit logs.
use crate::delivery::MailConfig;
use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    smtp::*,
};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use sqlx::{PgPool, Row};
use std::sync::Arc;
#[derive(Clone)]
pub struct PgSmtp {
    pub pool: PgPool,
    pub fallback: Option<Arc<MailConfig>>,
}
fn err(_: impl std::fmt::Debug) -> Error {
    Error::Unavailable
}
fn key() -> Result<aead::LessSafeKey> {
    let path =
        std::env::var("SMTP_ENCRYPTION_KEY_FILE").unwrap_or_else(|_| ".secrets/smtp-key".into());
    let raw = std::fs::read_to_string(path).map_err(err)?;
    let raw = raw.trim();
    if raw.len() != 64 {
        return Err(Error::Unavailable);
    }
    let bytes = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(raw.get(i..i + 2).ok_or(Error::Unavailable)?, 16).map_err(err))
        .collect::<Result<Vec<_>>>()?;
    Ok(aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_256_GCM, &bytes).map_err(err)?,
    ))
}
fn encrypt(value: &str) -> Result<Vec<u8>> {
    let key = key()?;
    let mut nonce = [0; 12];
    SystemRandom::new().fill(&mut nonce).map_err(err)?;
    let mut data = value.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(b"open-faktura-smtp-v1"),
        &mut data,
    )
    .map_err(err)?;
    let mut result = nonce.to_vec();
    result.extend(data);
    Ok(result)
}
fn decrypt(value: &[u8]) -> Result<String> {
    if value.len() < 28 {
        return Err(Error::Unavailable);
    }
    let nonce = value[..12].try_into().map_err(err)?;
    let mut data = value[12..].to_vec();
    let key = key()?;
    let plain = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(b"open-faktura-smtp-v1"),
            &mut data,
        )
        .map_err(err)?;
    String::from_utf8(plain.to_vec()).map_err(err)
}
async fn admin(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, actor: &str) -> Result<i64> {
    let id = actor.parse::<i64>().map_err(|_| Error::Forbidden)?;
    let allowed = sqlx::query_scalar::<_, bool>(
        "SELECT installation_admin FROM app_user WHERE id=$1 FOR SHARE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    .unwrap_or(false);
    if !allowed {
        return Err(Error::Forbidden);
    }
    Ok(id)
}
fn settings(row: &sqlx::postgres::PgRow) -> Settings {
    Settings {
        revision: row.get("revision"),
        mode: row.get("mode"),
        host: row.get("host"),
        port: row.get::<i32, _>("port") as u16,
        security: row.get("security"),
        sender: row.get("sender"),
        username: row.get("username"),
    }
}
fn config(
    row: &sqlx::postgres::PgRow,
    fallback: Option<Arc<MailConfig>>,
) -> Result<Option<Arc<MailConfig>>> {
    let s = settings(row);
    match s.mode.as_str() {
        "environment" => Ok(fallback),
        "disabled" => Ok(None),
        _ => {
            let encrypted: Option<Vec<u8>> = row.get("password");
            let password = encrypted.map(|p| decrypt(&p)).transpose()?;
            Ok(Some(Arc::new(
                MailConfig::configured(
                    &s.host,
                    s.port,
                    &s.security,
                    &s.sender,
                    &s.username,
                    password.as_deref(),
                )
                .map_err(|_| Error::Invalid(vec!["smtp".into()]))?,
            )))
        }
    }
}
pub async fn resolve(
    pool: &PgPool,
    fallback: Option<Arc<MailConfig>>,
) -> Result<Option<Arc<MailConfig>>> {
    let row = sqlx::query("SELECT * FROM smtp_settings WHERE id")
        .fetch_one(pool)
        .await
        .map_err(err)?;
    config(&row, fallback)
}
/// Resolve settings on an already borrowed connection, without exhausting the pool.
pub(crate) async fn resolve_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fallback: Option<Arc<MailConfig>>,
) -> Result<Option<Arc<MailConfig>>> {
    let row = sqlx::query("SELECT * FROM smtp_settings WHERE id")
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    config(&row, fallback)
}
#[async_trait]
impl SmtpStore for PgSmtp {
    async fn get(&self, actor: &str) -> Result<State> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        admin(&mut tx, actor).await?;
        let row = sqlx::query("SELECT * FROM smtp_settings WHERE id")
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        let configured = match row.get::<String, _>("mode").as_str() {
            "custom" => true,
            "environment" => self.fallback.is_some(),
            _ => false,
        };
        Ok(State {
            settings: settings(&row),
            password_set: row.get::<Option<Vec<u8>>, _>("password").is_some(),
            encryption_ready: key().is_ok(),
            configured,
        })
    }
    async fn save(&self, actor: &str, cmd: Save) -> Result<State> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let actor_id = admin(&mut tx, actor).await?;
        let row = sqlx::query("SELECT * FROM smtp_settings WHERE id FOR UPDATE")
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        let s = cmd.settings;
        if s.revision != row.get::<i64, _>("revision") {
            return Err(Error::Conflict);
        }
        let mut encrypted: Option<Vec<u8>> = row.get("password");
        if let Some(password) = cmd.password {
            encrypted = if password.is_empty() {
                None
            } else {
                Some(encrypt(&password)?)
            };
        }
        if s.username.is_empty() {
            encrypted = None;
        }
        if s.mode == "custom" {
            if s.host.is_empty() {
                return Err(Error::Invalid(vec!["host".into()]));
            }
            let password = encrypted.as_ref().map(|p| decrypt(p)).transpose()?;
            if !s.username.is_empty() && password.is_none() {
                return Err(Error::Invalid(vec!["password".into()]));
            }
            MailConfig::configured(
                &s.host,
                s.port,
                &s.security,
                &s.sender,
                &s.username,
                password.as_deref(),
            )
            .map_err(|_| Error::Invalid(vec!["smtp".into()]))?;
        }
        sqlx::query("UPDATE smtp_settings SET revision=revision+1,mode=$1,host=$2,port=$3,security=$4,sender=$5,username=$6,password=$7,updated_by=$8,updated_at=now() WHERE id").bind(s.mode).bind(s.host).bind(i32::from(s.port)).bind(s.security).bind(s.sender).bind(s.username).bind(encrypted).bind(actor_id).execute(&mut *tx).await.map_err(err)?;
        sqlx::query(
            "INSERT INTO installation_audit(user_id,action) VALUES($1,'smtp_settings_updated')",
        )
        .bind(actor_id)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        tx.commit().await.map_err(err)?;
        self.get(actor).await
    }
    async fn test(&self, actor: &str, cmd: Test) -> Result<TestResult> {
        cmd.recipient
            .parse::<lettre::Address>()
            .map_err(|_| Error::Invalid(vec!["recipient".into()]))?;
        let mut tx = self.pool.begin().await.map_err(err)?;
        let actor_id = admin(&mut tx, actor).await?;
        sqlx::query("UPDATE smtp_test SET status='uncertain',error_code='interrupted',finished_at=now() WHERE status='sending' AND created_at<now()-interval '2 minutes'").execute(&mut *tx).await.map_err(err)?;
        // Serialize registration only. No network I/O while holding this lock.
        let row = sqlx::query("SELECT * FROM smtp_settings WHERE id FOR UPDATE")
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        if let Some(old) = sqlx::query("SELECT * FROM smtp_test WHERE request_id=$1::text::uuid")
            .bind(&cmd.request_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
        {
            if old.get::<i64, _>("actor_id") != actor_id
                || old.get::<i64, _>("revision") != cmd.revision
                || old.get::<String, _>("recipient") != cmd.recipient
                || old.get::<String, _>("language") != cmd.language
            {
                return Err(Error::Conflict);
            }
            return Ok(TestResult {
                request_id: cmd.request_id,
                status: old.get("status"),
                error_code: old.get("error_code"),
            });
        }
        if row.get::<i64, _>("revision") != cmd.revision {
            return Err(Error::Conflict);
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM smtp_test WHERE created_at>now()-interval '1 minute'",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if count >= 5 {
            return Err(Error::Invalid(vec!["rate_limit".into()]));
        }
        let mail = config(&row, self.fallback.clone())?
            .ok_or_else(|| Error::Invalid(vec!["smtp".into()]))?;
        sqlx::query("INSERT INTO smtp_test(request_id,actor_id,revision,recipient,language,status) VALUES($1::text::uuid,$2,$3,$4,$5,'sending')").bind(&cmd.request_id).bind(actor_id).bind(cmd.revision).bind(&cmd.recipient).bind(&cmd.language).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        let (status, code) = mail
            .test_message(&cmd.recipient, &cmd.language, &cmd.request_id)
            .await;
        sqlx::query("UPDATE smtp_test SET status=$2,error_code=$3,finished_at=now() WHERE request_id=$1::text::uuid").bind(&cmd.request_id).bind(status).bind(code).execute(&self.pool).await.map_err(err)?;
        Ok(TestResult {
            request_id: cmd.request_id,
            status: status.into(),
            error_code: code.map(str::to_owned),
        })
    }
}

use async_trait::async_trait;
use base64::Engine;
use faktura_application::{bank_notification::*, banking::ImportPreview};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgNotification(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn bad() -> Error {
    Error::Invalid(vec!["bank_notification".into()])
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
async fn settings(tx: &mut Transaction<'_, Postgres>, c: i64, account: &str) -> Result<Settings> {
    crate::banking::get_account(tx, c, account).await?;
    let row = sqlx::query(
        "SELECT * FROM bank_notification_settings WHERE company_id=$1 AND account_id=$2",
    )
    .bind(c)
    .bind(id(account)?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?;
    Ok(row
        .map(|r| Settings {
            account_id: account.into(),
            revision: r.get("revision"),
            enabled: r.get("enabled"),
            format: r.get("format"),
        })
        .unwrap_or(Settings {
            account_id: account.into(),
            revision: 0,
            enabled: false,
            format: "camt053".into(),
        }))
}
#[async_trait]
impl NotificationStore for PgNotification {
    async fn settings(&self, a: &str, c: &str, account: &str) -> Result<Settings> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        settings(&mut tx, c, account).await
    }
    async fn configure(&self, a: &str, c: &str, value: Settings) -> Result<Settings> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        if !matches!(
            value.format.as_str(),
            "camt053" | "fio_json" | "moneta_xml" | "rb_abo"
        ) {
            return Err(bad());
        }
        let account = crate::banking::get_account(&mut tx, c, &value.account_id).await?;
        if account.archived {
            return Err(Error::Conflict);
        }
        if settings(&mut tx, c, &value.account_id).await?.revision != value.revision {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO bank_notification_settings(company_id,account_id,actor_id,enabled,format) VALUES($1,$2,$3,$4,$5) ON CONFLICT(company_id,account_id) DO UPDATE SET actor_id=$3,enabled=$4,format=$5,revision=bank_notification_settings.revision+1,updated_at=now()")
            .bind(c).bind(id(&value.account_id)?).bind(a).bind(value.enabled).bind(value.format).execute(&mut *tx).await.map_err(err)?;
        let result = settings(&mut tx, c, &value.account_id).await?;
        sqlx::query("INSERT INTO bank_notification_settings_audit(company_id,account_id,actor_id,revision,enabled,format) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(c).bind(id(&result.account_id)?).bind(a).bind(result.revision).bind(result.enabled).bind(&result.format).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn history(&self, a: &str, c: &str, account: &str) -> Result<Vec<Diagnostic>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        crate::banking::get_account(&mut tx, c, account).await?;
        let rows=sqlx::query("SELECT message_id,sender,new_count,duplicate_count,created_at FROM bank_notification WHERE company_id=$1 AND account_id=$2 ORDER BY created_at DESC,message_id DESC LIMIT 100").bind(c).bind(id(account)?).fetch_all(&mut *tx).await.map_err(err)?;
        Ok(rows
            .into_iter()
            .map(|r| Diagnostic {
                message_id: r.get("message_id"),
                sender: r.get("sender"),
                new_count: r.get("new_count"),
                duplicate_count: r.get("duplicate_count"),
                created_at: r
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
            })
            .collect())
    }
    async fn receive(&self, a: &str, c: &str, input: Intake) -> Result<ImportPreview> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        uuid::Uuid::parse_str(&input.request_id).map_err(|_| bad())?;
        if input.message_id.is_empty()
            || input.message_id.len() > 998
            || input.sender.len() > 320
            || [&input.message_id, &input.sender]
                .iter()
                .any(|v| v.chars().any(char::is_control))
            || input.file.filename.len() > 180
            || input.file.content_base64.len() > 2796204
        {
            return Err(bad());
        }
        let setting = settings(&mut tx, c, &input.file.account_id).await?;
        if !setting.enabled {
            return Err(Error::Forbidden);
        }
        if setting.format != input.file.format {
            return Err(bad());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&input.file.content_base64)
            .map_err(|_| bad())?;
        if bytes.len() > 2097152 {
            return Err(bad());
        }
        let hash=format!("{:x}",Sha256::digest(serde_json::to_vec(&serde_json::json!({"account":input.file.account_id,"sender":input.sender,"format":input.file.format,"filename":input.file.filename,"sha256":format!("{:x}",Sha256::digest(&bytes))})).map_err(err)?));
        let previous=sqlx::query("SELECT * FROM bank_notification WHERE company_id=$1 AND ((account_id=$2 AND message_id=$3) OR request_id=$4::text::uuid)").bind(c).bind(id(&input.file.account_id)?).bind(&input.message_id).bind(&input.request_id).fetch_all(&mut *tx).await.map_err(err)?;
        if previous.len() > 1 {
            return Err(Error::Conflict);
        }
        if let Some(r) = previous.into_iter().next() {
            if r.get::<String, _>("input_hash") != hash
                || r.get::<String, _>("message_id") != input.message_id
            {
                return Err(Error::Conflict);
            }
            return serde_json::from_value(r.get("result")).map_err(err);
        }
        let account = id(&input.file.account_id)?;
        let report = crate::banking::statement_import_in_tx(&mut tx, c, input.file).await?;
        sqlx::query("INSERT INTO bank_notification(company_id,account_id,message_id,request_id,actor_id,sender,input_hash,result,new_count,duplicate_count) VALUES($1,$2,$3,$4::text::uuid,$5,$6,$7,$8,$9,$10)").bind(c).bind(account).bind(input.message_id).bind(input.request_id).bind(a).bind(input.sender).bind(hash).bind(serde_json::to_value(&report).map_err(err)?).bind(report.new_count as i32).bind(report.duplicate_count as i32).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(report)
    }
}

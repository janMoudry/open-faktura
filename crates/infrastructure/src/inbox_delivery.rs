use crate::delivery::MailConfig;
use async_trait::async_trait;
use faktura_application::inbox_delivery::*;
use lettre::{
    message::{
        header::{ContentTransferEncoding, ContentType},
        Attachment, Body, MultiPart, SinglePart,
    },
    Message,
};
use sqlx::{PgPool, Row};
use std::sync::Arc;
#[derive(Clone)]
pub struct PgInboxDelivery {
    pub pool: PgPool,
    pub mail: Option<Arc<MailConfig>>,
}
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn map(r: sqlx::postgres::PgRow) -> Delivery {
    Delivery {
        id: r.get::<i64, _>("id").to_string(),
        document_id: r.get::<i64, _>("document_id").to_string(),
        revision: r.get("revision"),
        recipient: r.get("recipient"),
        subject: r.get("subject"),
        body: r.get("body"),
        filename: r.get("filename"),
        sha256: r.get("sha256"),
        size: r.get("size"),
        sender: r.get("sender"),
        status: r.get("status"),
        error_code: r.get("error_code"),
        created_at: r
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
        finished_at: r
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
            .map(|d| d.to_rfc3339()),
    }
}
#[async_trait]
impl InboxDeliveryStore for PgInboxDelivery {
    async fn history(&self, a: &str, c: &str, d: &str) -> Result<History> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM inbox_document WHERE company_id=$1 AND id=$2)",
        )
        .bind(c)
        .bind(id(d)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if !exists {
            return Err(Error::NotFound);
        }
        let rows=sqlx::query("SELECT id,document_id,revision,recipient,subject,body,filename,sha256,size,sender,status,error_code,created_at,finished_at FROM inbox_delivery WHERE company_id=$1 AND document_id=$2 ORDER BY id DESC LIMIT 50").bind(c).bind(id(d)?).fetch_all(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        let mail = crate::smtp::resolve(&self.pool, self.mail.clone()).await?;
        Ok(History {
            configured: mail.is_some(),
            sender: mail.map(|m| m.sender.clone()),
            items: rows.into_iter().map(map).collect(),
        })
    }
    async fn send(&self, a: &str, c: &str, d: &str, v: Send) -> Result<Delivery> {
        v.validate()?;
        v.recipient
            .parse::<lettre::Address>()
            .map_err(|_| Error::Invalid(vec!["recipient".into()]))?;
        let command = serde_json::to_value(&v).map_err(err)?;
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT id,document_id,revision,recipient,subject,body,filename,sha256,size,sender,status,error_code,created_at,finished_at,actor_id,command FROM inbox_delivery WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&v.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != a
                || r.get::<i64, _>("document_id") != id(d)?
                || r.get::<serde_json::Value, _>("command") != command
            {
                return Err(Error::Conflict);
            }
            return Ok(map(r));
        }
        let r = sqlx::query("SELECT * FROM inbox_document WHERE company_id=$1 AND id=$2 FOR SHARE")
            .bind(c)
            .bind(id(d)?)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
        if r.get::<i64, _>("revision") != v.revision {
            return Err(Error::Conflict);
        }
        let mail = crate::smtp::resolve_in_tx(&mut tx, self.mail.clone())
            .await?
            .ok_or_else(|| Error::Invalid(vec!["smtp".into()]))?;
        let message_id = format!(
            "{}@{}",
            uuid::Uuid::new_v4(),
            mail.sender.split('@').next_back().unwrap_or("localhost")
        );
        let row=sqlx::query("INSERT INTO inbox_delivery(company_id,document_id,actor_id,request_id,command,revision,sender,recipient,subject,body,message_id,filename,media_type,sha256,size,content) VALUES($1,$2,$3,$4::text::uuid,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING *").bind(c).bind(id(d)?).bind(a).bind(&v.request_id).bind(command).bind(v.revision).bind(&mail.sender).bind(v.recipient).bind(v.subject).bind(v.body).bind(message_id).bind(r.get::<String,_>("filename")).bind(r.get::<String,_>("media_type")).bind(r.get::<String,_>("sha256")).bind(r.get::<i64,_>("size")).bind(r.get::<Vec<u8>,_>("content")).fetch_one(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(map(row))
    }
}
impl PgInboxDelivery {
    pub async fn process_one(&self) -> Result<bool> {
        let Some(mail) = crate::smtp::resolve(&self.pool, self.mail.clone()).await? else {
            return Ok(false);
        };
        sqlx::query("UPDATE inbox_delivery SET status='uncertain',error_code='interrupted',finished_at=now() WHERE status='sending' AND started_at<now()-interval '2 minutes'").execute(&self.pool).await.map_err(err)?;
        let row=sqlx::query("UPDATE inbox_delivery SET status='sending',started_at=now() WHERE id=(SELECT id FROM inbox_delivery WHERE status='queued' ORDER BY id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *").fetch_optional(&self.pool).await.map_err(err)?;
        let Some(row) = row else { return Ok(false) };
        // Rights are checked immediately before dispatch, outside SMTP. Revocation
        // after this boundary cannot recall a message already accepted by SMTP.
        let permitted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND m.role IN ('owner','editor') AND c.deleted_at IS NULL)").bind(row.get::<i64,_>("company_id")).bind(row.get::<i64,_>("actor_id")).fetch_one(&self.pool).await.map_err(err)?;
        let (status, code) = if !permitted {
            ("failed", Some("permission_changed"))
        } else {
            let message = (|| {
                let mime: ContentType = row.get::<String, _>("media_type").parse().ok()?;
                Message::builder()
                    .from(row.get::<String, _>("sender").parse().ok()?)
                    .to(row.get::<String, _>("recipient").parse().ok()?)
                    .subject(row.get::<String, _>("subject"))
                    .message_id(Some(row.get("message_id")))
                    .multipart(
                        MultiPart::mixed()
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_PLAIN)
                                    .body(row.get::<String, _>("body")),
                            )
                            .singlepart(
                                Attachment::new(row.get::<String, _>("filename")).body(
                                    Body::new_with_encoding(
                                        row.get::<Vec<u8>, _>("content"),
                                        ContentTransferEncoding::Base64,
                                    )
                                    .ok()?,
                                    mime,
                                ),
                            ),
                    )
                    .ok()
            })();
            match message {
                Some(m) => mail.dispatch_message(m).await,
                None => ("failed", Some("message_invalid")),
            }
        };
        sqlx::query("UPDATE inbox_delivery SET status=$2,error_code=$3,finished_at=now() WHERE id=$1 AND status='sending'").bind(row.get::<i64,_>("id")).bind(status).bind(code).execute(&self.pool).await.map_err(err)?;
        Ok(true)
    }
}

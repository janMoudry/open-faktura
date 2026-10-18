//! Issued invoice outbox; never changes invoice revision, numbering or settlement.
use crate::{
    delivery::{map, MailConfig},
    draft::{authorize, PgDraft},
};
use async_trait::async_trait;
use faktura_application::{delivery::*, draft::DraftStore};
use sqlx::{PgPool, Row};
use std::sync::Arc;
#[derive(Clone)]
pub struct PgInvoiceDelivery {
    pub pool: PgPool,
    pub mail: Option<Arc<MailConfig>>,
}
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
const ROWS: &str = "SELECT * FROM invoice_delivery";
fn replay(r: sqlx::postgres::PgRow, a: i64, d: i64, cmd: &serde_json::Value) -> Result<Delivery> {
    if r.get::<i64, _>("actor_id") != a
        || r.get::<i64, _>("document_id") != d
        || r.get::<serde_json::Value, _>("command") != *cmd
    {
        return Err(Error::Conflict);
    }
    Ok(map(r))
}
#[async_trait]
impl DeliveryStore for PgInvoiceDelivery {
    async fn pdf(&self, a: &str, c: &str, d: &str, delivery: Option<&str>) -> Result<Vec<u8>> {
        if let Some(delivery) = delivery {
            let mut tx = self.pool.begin().await.map_err(err)?;
            let (_, c) = authorize(&mut tx, a, c, false).await?;
            return sqlx::query_scalar(
                "SELECT pdf FROM invoice_delivery WHERE company_id=$1 AND document_id=$2 AND id=$3",
            )
            .bind(c)
            .bind(id(d)?)
            .bind(id(delivery)?)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound);
        }
        PgDraft(self.pool.clone()).pdf(a, c, d).await
    }
    async fn history(&self, a: &str, c: &str, d: &str, page: i64) -> Result<History> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let row = sqlx::query("SELECT number FROM invoice_draft WHERE company_id=$1 AND id=$2")
            .bind(c)
            .bind(id(d)?)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
        if row.get::<Option<String>, _>("number").is_none() {
            return Err(Error::Conflict);
        }
        let total = sqlx::query_scalar(
            "SELECT count(*) FROM invoice_delivery WHERE company_id=$1 AND document_id=$2",
        )
        .bind(c)
        .bind(id(d)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items = sqlx::query(&format!(
            "{ROWS} WHERE company_id=$1 AND document_id=$2 ORDER BY id DESC LIMIT 25 OFFSET $3"
        ))
        .bind(c)
        .bind(id(d)?)
        .bind((page - 1) * 25)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?
        .into_iter()
        .map(map)
        .collect();
        let mail = crate::smtp::resolve(&self.pool, self.mail.clone()).await?;
        Ok(History {
            configured: mail.is_some(),
            sender: mail.as_ref().map(|m| m.sender.clone()),
            items,
            total,
            page,
        })
    }
    async fn send(&self, a: &str, c: &str, d: &str, cmd: SendQuote) -> Result<Delivery> {
        cmd.recipient
            .parse::<lettre::Address>()
            .map_err(|_| Error::Invalid(vec!["recipient".into()]))?;
        let mut command = serde_json::to_value(&cmd).map_err(err)?;
        if cmd.share_id.is_none() {
            command
                .as_object_mut()
                .ok_or(Error::Unavailable)?
                .remove("share_id");
        }
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (actor, company) = authorize(&mut tx, a, c, true).await?;
        if let Some(row) = sqlx::query(&format!(
            "{ROWS} WHERE company_id=$1 AND request_id=$2::text::uuid"
        ))
        .bind(company)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            return replay(row, actor, id(d)?, &command);
        }
        tx.commit().await.map_err(err)?;
        let mail = crate::smtp::resolve(&self.pool, self.mail.clone())
            .await?
            .ok_or_else(|| Error::Invalid(vec!["smtp".into()]))?;
        let invoice = PgDraft(self.pool.clone()).get(a, c, d).await?;
        if invoice.number.is_none()
            || invoice.revision != cmd.revision
            || invoice.data.calculation.calculation_version == "import-v1"
        {
            return Err(Error::Conflict);
        }
        let reply_to = invoice.data.supplier.email.clone();
        if !reply_to.is_empty() {
            reply_to
                .parse::<lettre::Address>()
                .map_err(|_| Error::Invalid(vec!["supplier.email".into()]))?;
        }
        let snapshot = serde_json::to_value(&invoice).map_err(err)?;
        // Use the exact cached issued PDF, rendered without a DB transaction if needed.
        let pdf = PgDraft(self.pool.clone()).pdf(a, c, d).await?;
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (actor, company) = authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(company)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        if let Some(row) = sqlx::query(&format!(
            "{ROWS} WHERE company_id=$1 AND request_id=$2::text::uuid"
        ))
        .bind(company)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            return replay(row, actor, id(d)?, &command);
        }
        let current = crate::draft::map(
            sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR SHARE")
                .bind(company)
                .bind(id(d)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
        )?;
        if current.number.is_none() || current.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        if let Some(purpose) = &cmd.purpose {
            if !crate::communication::eligible(
                &mut tx,
                company,
                id(d)?,
                purpose,
                cmd.settlement_revision.ok_or(Error::Conflict)?,
            )
            .await?
            {
                return Err(Error::Conflict);
            }
            if let Some(revision) = cmd.automation_revision {
                if !crate::communication::automated(&mut tx, company, id(d)?, purpose, revision)
                    .await?
                {
                    return Err(Error::Conflict);
                }
            }
        }
        let link_url: Option<String> = if let Some(share) = &cmd.share_id {
            Some(sqlx::query_scalar("SELECT url FROM invoice_share WHERE company_id=$1 AND document_id=$2 AND id=$3 AND revoked_at IS NULL AND expires_at>now() FOR SHARE").bind(company).bind(id(d)?).bind(id(share)?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?)
        } else {
            None
        };
        let body = if let Some(url) = link_url {
            format!("{}\n\n{}", cmd.body, url)
        } else {
            cmd.body.clone()
        };
        let message_id = format!(
            "{}@{}",
            uuid::Uuid::new_v4(),
            mail.sender.split('@').next_back().unwrap_or("localhost")
        );
        let files =
            crate::attachment::snapshot(&mut tx, company, "invoice", id(d)?, &cmd.attachment_ids)
                .await?;
        let row=sqlx::query("INSERT INTO invoice_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id,attachments,purpose,settlement_revision,automation_revision,share_id) VALUES($1,$2,$3,$4::text::uuid,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19) RETURNING *").bind(company).bind(id(d)?).bind(actor).bind(&cmd.request_id).bind(command).bind(cmd.revision).bind(snapshot).bind(pdf).bind(&mail.sender).bind(reply_to).bind(cmd.recipient).bind(cmd.subject).bind(body).bind(message_id).bind(serde_json::to_value(&files).map_err(err)?).bind(cmd.purpose.as_deref().unwrap_or("document")).bind(cmd.settlement_revision).bind(cmd.automation_revision).bind(cmd.share_id.as_deref().map(id).transpose()?).fetch_one(&mut *tx).await.map_err(err)?;
        crate::attachment::link(&mut tx, company, "invoice", row.get("id"), &files).await?;
        tx.commit().await.map_err(err)?;
        Ok(map(row))
    }
}
impl PgInvoiceDelivery {
    pub async fn process_one(&self) -> Result<bool> {
        let Some(mail) = crate::smtp::resolve(&self.pool, self.mail.clone()).await? else {
            return Ok(false);
        };
        sqlx::query("UPDATE invoice_delivery SET status='uncertain',error_code='interrupted',finished_at=now() WHERE status='sending' AND started_at<now()-interval '2 minutes'").execute(&self.pool).await.map_err(err)?;
        sqlx::query("UPDATE invoice_delivery d SET status='failed',error_code='profile_unavailable',finished_at=now() WHERE status='queued' AND NOT EXISTS(SELECT 1 FROM company c WHERE c.id=d.company_id AND c.deleted_at IS NULL)").execute(&self.pool).await.map_err(err)?;
        let row=sqlx::query("UPDATE invoice_delivery SET status='sending',started_at=now() WHERE id=(SELECT id FROM invoice_delivery d WHERE status='queued' AND EXISTS(SELECT 1 FROM company c WHERE c.id=d.company_id AND c.deleted_at IS NULL) ORDER BY id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *").fetch_optional(&self.pool).await.map_err(err)?;
        let Some(row) = row else { return Ok(false) };
        if row.get::<String, _>("purpose") != "document" {
            let mut tx = self.pool.begin().await.map_err(err)?;
            let c: i64 = row.get("company_id");
            let i: i64 = row.get("document_id");
            sqlx::query("SELECT pg_advisory_xact_lock($1)")
                .bind(c)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            sqlx::query("SELECT id FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR SHARE")
                .bind(c)
                .bind(i)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
            let purpose: String = row.get("purpose");
            let mut valid = crate::communication::eligible(
                &mut tx,
                c,
                i,
                &purpose,
                row.get("settlement_revision"),
            )
            .await?;
            if let Some(revision) = row.get::<Option<i64>, _>("automation_revision") {
                valid &= crate::communication::automated(&mut tx, c, i, &purpose, revision).await?;
            }
            let permitted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM membership m JOIN company c ON c.id=m.company_id WHERE m.company_id=$1 AND m.user_id=$2 AND m.role IN ('owner','editor') AND c.deleted_at IS NULL)").bind(c).bind(row.get::<i64,_>("actor_id")).fetch_one(&mut *tx).await.map_err(err)?;
            if !valid || !permitted {
                sqlx::query("UPDATE invoice_delivery SET status='failed',error_code='communication_changed',finished_at=now() WHERE id=$1 AND status='sending'").bind(row.get::<i64,_>("id")).execute(&mut *tx).await.map_err(err)?;
                tx.commit().await.map_err(err)?;
                return Ok(true);
            }
            // This is the dispatch eligibility boundary. SMTP is intentionally outside DB locks.
            tx.commit().await.map_err(err)?;
        }
        if let Some(share) = row.get::<Option<i64>, _>("share_id") {
            let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invoice_share WHERE company_id=$1 AND document_id=$2 AND id=$3 AND revoked_at IS NULL AND expires_at>now())").bind(row.get::<i64,_>("company_id")).bind(row.get::<i64,_>("document_id")).bind(share).fetch_one(&self.pool).await.map_err(err)?;
            if !valid {
                sqlx::query("UPDATE invoice_delivery SET status='failed',error_code='link_unavailable',finished_at=now() WHERE id=$1 AND status='sending'").bind(row.get::<i64,_>("id")).execute(&self.pool).await.map_err(err)?;
                return Ok(true);
            }
        }
        let files =
            crate::attachment::queued(&self.pool, row.get("company_id"), "invoice", row.get("id"))
                .await?;
        let (status, code) = mail.deliver(&row, files).await;
        sqlx::query("UPDATE invoice_delivery SET status=$2,error_code=$3,finished_at=now() WHERE id=$1 AND status='sending'").bind(row.get::<i64,_>("id")).bind(status).bind(code).execute(&self.pool).await.map_err(err)?;
        Ok(true)
    }
}

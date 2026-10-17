use crate::{
    draft::{authorize, PgDraft},
    invoice_delivery::PgInvoiceDelivery,
};
use async_trait::async_trait;
use faktura_application::{
    communication::*,
    delivery::{self, SendQuote},
    draft::DraftStore,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgCommunication(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn settings(r: sqlx::postgres::PgRow) -> Settings {
    Settings {
        revision: r.get("revision"),
        reminders: r.get("reminders"),
        thanks: r.get("thanks"),
        first_days: r.get("first_days"),
        repeat_days: r.get("repeat_days"),
        minimum_total: r.get("minimum_total"),
    }
}
const SETTINGS:&str="SELECT revision,reminders,thanks,first_days,repeat_days,minimum_total::text FROM communication_settings";
/// Called with the invoice row locked, serializing against payments and credit offsets.
pub(crate) async fn eligible(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    i: i64,
    purpose: &str,
    revision: i64,
) -> Result<bool> {
    sqlx::query_scalar("SELECT payment_revision=$3 AND CASE $4 WHEN 'reminder' THEN overdue WHEN 'thanks' THEN outstanding=0 AND cash_paid>0 ELSE false END FROM invoice_settlement WHERE company_id=$1 AND invoice_id=$2")
        .bind(c).bind(i).bind(revision).bind(purpose).fetch_optional(&mut **tx).await.map_err(err).map(|v|v.unwrap_or(false))
}
pub(crate) async fn automated(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    i: i64,
    purpose: &str,
    revision: i64,
) -> Result<bool> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM communication_settings s JOIN membership m ON m.company_id=s.company_id AND m.user_id=s.actor_id WHERE s.company_id=$1 AND s.revision=$4 AND m.role IN ('owner','editor') AND CASE $3 WHEN 'reminder' THEN s.reminders WHEN 'thanks' THEN s.thanks ELSE false END AND NOT EXISTS(SELECT 1 FROM communication_pause p WHERE p.company_id=$1 AND p.invoice_id=$2 AND p.paused))")
        .bind(c).bind(i).bind(purpose).bind(revision).fetch_one(&mut **tx).await.map_err(err)
}
#[async_trait]
impl CommunicationStore for PgCommunication {
    async fn settings(&self, a: &str, c: &str) -> Result<Settings> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        Ok(sqlx::query(&format!("{SETTINGS} WHERE company_id=$1"))
            .bind(c)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .map(settings)
            .unwrap_or_default())
    }
    async fn save(&self, a: &str, c: &str, v: Settings) -> Result<Settings> {
        v.validate()?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        let owner: bool = sqlx::query_scalar(
            "SELECT role='owner' FROM membership WHERE company_id=$1 AND user_id=$2",
        )
        .bind(c)
        .bind(a)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if !owner {
            return Err(Error::Forbidden);
        }
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let current = sqlx::query(&format!("{SETTINGS} WHERE company_id=$1 FOR UPDATE"))
            .bind(c)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .map(settings)
            .unwrap_or_default();
        if current.revision != v.revision {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO communication_settings(company_id,actor_id,reminders,thanks,first_days,repeat_days,minimum_total) VALUES($1,$2,$3,$4,$5,$6,$7::text::numeric) ON CONFLICT(company_id) DO UPDATE SET revision=communication_settings.revision+1,actor_id=$2,reminders=$3,thanks=$4,first_days=$5,repeat_days=$6,minimum_total=$7::text::numeric,thanks_since=CASE WHEN NOT communication_settings.thanks AND $4 THEN now() ELSE communication_settings.thanks_since END,updated_at=now()")
            .bind(c).bind(a).bind(v.reminders).bind(v.thanks).bind(v.first_days).bind(v.repeat_days).bind(&v.minimum_total).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("INSERT INTO communication_audit(company_id,actor_id,input) VALUES($1,$2,$3)")
            .bind(c)
            .bind(a)
            .bind(serde_json::to_value(&v).map_err(err)?)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(Settings {
            revision: v.revision + 1,
            ..v
        })
    }
    async fn pause(&self, a: &str, c: &str, i: &str, v: Pause) -> Result<Pause> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        let i = id(i)?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invoice_draft WHERE company_id=$1 AND id=$2 AND number IS NOT NULL)").bind(c).bind(i).fetch_one(&mut *tx).await.map_err(err)?;
        if !exists {
            return Err(Error::NotFound);
        }
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        sqlx::query("INSERT INTO communication_pause(company_id,invoice_id,paused,actor_id) VALUES($1,$2,$3,$4) ON CONFLICT(company_id,invoice_id) DO UPDATE SET paused=$3,actor_id=$4,updated_at=now()")
            .bind(c).bind(i).bind(v.paused).bind(a).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("INSERT INTO communication_audit(company_id,invoice_id,actor_id,input) VALUES($1,$2,$3,$4)").bind(c).bind(i).bind(a).bind(serde_json::to_value(&v).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
    async fn preview(&self, a: &str, c: &str, i: &str, purpose: &str) -> Result<Preview> {
        if !matches!(purpose, "reminder" | "thanks") {
            return Err(Error::Invalid(vec!["purpose".into()]));
        }
        let invoice = PgDraft(self.0.clone()).get(a, c, i).await?;
        if invoice.number.is_none() || invoice.data.calculation.calculation_version == "import-v1" {
            return Err(Error::Conflict);
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, company) = authorize(&mut tx, a, c, false).await?;
        let row=sqlx::query("SELECT payment_revision,outstanding::text FROM invoice_settlement WHERE company_id=$1 AND invoice_id=$2").bind(company).bind(id(i)?).fetch_one(&mut *tx).await.map_err(err)?;
        let revision = row.get("payment_revision");
        let valid = eligible(&mut tx, company, id(i)?, purpose, revision).await?;
        let paused:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM communication_pause WHERE company_id=$1 AND invoice_id=$2 AND paused)").bind(company).bind(id(i)?).fetch_one(&mut *tx).await.map_err(err)?;
        let catalog: serde_json::Value = serde_json::from_str(if invoice.data.language == "en" {
            include_str!("../assets/communication.en.json")
        } else {
            include_str!("../assets/communication.cs.json")
        })
        .map_err(err)?;
        let number = invoice.number.as_deref().unwrap_or_default();
        let amount: String = row.get("outstanding");
        let amount = if invoice.data.language == "cs" {
            amount.replace('.', ",")
        } else {
            amount
        };
        let due = invoice
            .data
            .due_on
            .map(|d| {
                d.format(if invoice.data.language == "cs" {
                    "%d. %m. %Y"
                } else {
                    "%d/%m/%Y"
                })
                .to_string()
            })
            .unwrap_or_default();
        let fill = |key: &str| {
            catalog[purpose][key]
                .as_str()
                .unwrap_or_default()
                .replace("{number}", number)
                .replace("{outstanding}", &amount)
                .replace("{currency}", &invoice.data.calculation.currency)
                .replace("{supplier}", &invoice.data.supplier.name)
                .replace("{due_on}", &due)
        };
        let automation_error:Option<String>=sqlx::query_scalar("SELECT error_code FROM communication_job WHERE company_id=$1 AND invoice_id=$2 ORDER BY created_at DESC LIMIT 1").bind(company).bind(id(i)?).fetch_optional(&mut *tx).await.map_err(err)?.flatten();
        Ok(Preview {
            automation_error,
            purpose: purpose.into(),
            eligible: valid,
            paused,
            settlement_revision: revision,
            revision: invoice.revision,
            recipient: invoice.data.customer.map(|v| v.email).unwrap_or_default(),
            subject: fill("subject"),
            body: fill("body"),
        })
    }
}
impl PgCommunication {
    /// Durable intents are unique per invoice/slot, so concurrent workers and restarts replay one outbox command.
    pub async fn process_one(&self, mail: &PgInvoiceDelivery) -> Result<bool> {
        if crate::smtp::resolve(&self.0, mail.mail.clone())
            .await?
            .is_none()
        {
            return Ok(false);
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(14002,1)")
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        if !locked {
            return Ok(false);
        }
        let candidate=sqlx::query(r#"SELECT d.company_id,d.id,s.actor_id,s.revision,
 CASE WHEN z.overdue AND s.reminders THEN 'reminder' ELSE 'thanks' END AS purpose,
 (SELECT count(*)::int FROM communication_job j WHERE j.company_id=d.company_id AND j.invoice_id=d.id AND j.purpose='reminder') AS slot
 FROM communication_settings s JOIN company c ON c.id=s.company_id AND c.deleted_at IS NULL
 JOIN membership m ON m.company_id=c.id AND m.user_id=s.actor_id AND m.role IN ('owner','editor')
 JOIN invoice_draft d ON d.company_id=c.id JOIN invoice_settlement z ON z.company_id=c.id AND z.invoice_id=d.id
 WHERE d.data#>>'{calculation,calculation_version}'<>'import-v1' AND COALESCE(d.data#>>'{customer,email}','')<>''
 AND NOT EXISTS(SELECT 1 FROM communication_pause p WHERE p.company_id=c.id AND p.invoice_id=d.id AND p.paused)
 AND ((s.reminders AND z.overdue AND z.today>=z.due_on+s.first_days AND z.payable>s.minimum_total
 AND (SELECT count(*) FROM communication_job j WHERE j.company_id=c.id AND j.invoice_id=d.id AND j.purpose='reminder')<10
 AND NOT EXISTS(SELECT 1 FROM communication_job j WHERE j.company_id=c.id AND j.invoice_id=d.id AND j.purpose='reminder' AND (j.created_at AT TIME ZONE (c.settings->>'timezone'))::date+s.repeat_days>z.today))
 OR (s.thanks AND z.outstanding=0 AND z.cash_paid>0
 AND EXISTS(SELECT 1 FROM invoice_payment p WHERE p.company_id=c.id AND p.invoice_id=d.id AND p.created_at>=s.thanks_since AND NOT EXISTS(SELECT 1 FROM invoice_payment_reversal r WHERE r.company_id=c.id AND r.payment_id=p.id))
 AND NOT EXISTS(SELECT 1 FROM communication_job j WHERE j.company_id=c.id AND j.invoice_id=d.id AND j.purpose='thanks')
 AND NOT EXISTS(SELECT 1 FROM invoice_delivery e WHERE e.company_id=c.id AND e.document_id=d.id AND e.purpose='thanks')))
 ORDER BY d.id LIMIT 1"#).fetch_optional(&mut *tx).await.map_err(err)?;
        if let Some(r) = candidate {
            let a = r.get::<i64, _>("actor_id").to_string();
            let c = r.get::<i64, _>("company_id").to_string();
            let i = r.get::<i64, _>("id").to_string();
            let purpose: String = r.get("purpose");
            let p = self.preview(&a, &c, &i, &purpose).await?;
            let cmd = SendQuote {
                purpose: Some(purpose.clone()),
                settlement_revision: Some(p.settlement_revision),
                automation_revision: Some(r.get("revision")),
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: p.revision,
                recipient: p.recipient,
                subject: p.subject,
                body: p.body,
                attachment_ids: vec![],
                share_id: None,
            };
            sqlx::query("INSERT INTO communication_job(company_id,invoice_id,purpose,slot,request_id,settings_revision,command,actor_id) VALUES($1,$2,$3,$4,$5::text::uuid,$6,$7,$8) ON CONFLICT DO NOTHING")
                .bind(id(&c)?).bind(id(&i)?).bind(&purpose).bind(if purpose=="thanks"{0}else{r.get::<i32,_>("slot")}).bind(&cmd.request_id).bind(r.get::<i64,_>("revision")).bind(serde_json::to_value(&cmd).map_err(err)?).bind(id(&a)?).execute(&mut *tx).await.map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        let job=sqlx::query("SELECT * FROM communication_job WHERE delivery_id IS NULL AND error_code IS NULL AND next_attempt<=now() ORDER BY created_at LIMIT 1").fetch_optional(&self.0).await.map_err(err)?;
        let Some(j) = job else { return Ok(false) };
        let cmd: SendQuote = serde_json::from_value(j.get("command")).map_err(err)?;
        let key = cmd.request_id.clone();
        let result = delivery::send(
            mail,
            &j.get::<i64, _>("actor_id").to_string(),
            &j.get::<i64, _>("company_id").to_string(),
            &j.get::<i64, _>("invoice_id").to_string(),
            cmd,
        )
        .await;
        match result {
            Ok(d) => {
                sqlx::query(
                    "UPDATE communication_job SET delivery_id=$2 WHERE request_id=$1::text::uuid",
                )
                .bind(key)
                .bind(id(&d.id)?)
                .execute(&self.0)
                .await
                .map_err(err)?;
            }
            Err(Error::Unavailable) => {
                sqlx::query("UPDATE communication_job SET next_attempt=now()+interval '5 minutes' WHERE request_id=$1::text::uuid").bind(key).execute(&self.0).await.map_err(err)?;
                return Err(Error::Unavailable);
            }
            Err(_) => {
                sqlx::query("UPDATE communication_job SET error_code='communication_changed' WHERE request_id=$1::text::uuid").bind(key).execute(&self.0).await.map_err(err)?;
            }
        }
        Ok(true)
    }
}

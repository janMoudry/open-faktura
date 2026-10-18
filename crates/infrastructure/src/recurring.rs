use crate::draft::{authorize, check_contact, create_in_tx, issue_in_tx};
use async_trait::async_trait;
use faktura_application::{
    delivery::SendQuote,
    draft::{Create as CreateDraft, Issue},
    recurring::*,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgRecurring(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn error_code(e: &Error) -> &'static str {
    match e {
        Error::Forbidden | Error::NotFound => "permission",
        Error::Invalid(fields)
            if fields
                .iter()
                .any(|f| f == "number_year" || f == "next_number") =>
        {
            "numbering"
        }
        Error::Invalid(fields) if fields.iter().any(|f| f == "tax_mode") => "tax_mode",
        Error::Invalid(fields) if fields.iter().any(|f| f == "smtp") => "smtp",
        Error::Invalid(_) => "validation",
        Error::Conflict => "conflict",
        _ => "unavailable",
    }
}
async fn lock(tx: &mut Transaction<'_, Postgres>, a: &str, c: &str) -> Result<(i64, i64)> {
    sqlx::query("SET LOCAL lock_timeout='5s'")
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    sqlx::query("SELECT id FROM company WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(id(c)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    authorize(tx, a, c, true).await
}
async fn load(tx: &mut Transaction<'_, Postgres>, c: i64, i: i64) -> Result<Rule> {
    let r = sqlx::query("SELECT * FROM recurring_rule WHERE company_id=$1 AND id=$2")
        .bind(c)
        .bind(i)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    let data: Data = serde_json::from_value(r.get("data")).map_err(err)?;
    let generated: i32 = r.get("generated");
    let occurrences=sqlx::query("SELECT o.*,i.number,d.status AS delivery_status FROM recurring_occurrence o JOIN invoice_draft i ON i.company_id=o.company_id AND i.id=o.invoice_id LEFT JOIN invoice_delivery d ON d.company_id=o.company_id AND d.id=o.delivery_id WHERE o.company_id=$1 AND o.rule_id=$2 ORDER BY o.index DESC LIMIT 100").bind(c).bind(i).fetch_all(&mut **tx).await.map_err(err)?.into_iter().map(|o|Occurrence{index:o.get("index"),scheduled_on:o.get::<chrono::NaiveDate,_>("scheduled_on").to_string(),invoice_id:o.get::<i64,_>("invoice_id").to_string(),number:o.get("number"),delivery_status:o.get::<Option<String>,_>("delivery_status").or_else(||o.get::<Option<serde_json::Value>,_>("mail_command").map(|_|"pending".into())),error_code:o.get("error_code")}).collect();
    Ok(Rule {
        id: i.to_string(),
        company_id: c.to_string(),
        revision: r.get("revision"),
        status: r.get("status"),
        generated,
        error_code: r.get("error_code"),
        next_dates: data.schedule.preview(generated as u32),
        data,
        occurrences,
    })
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    v: &Rule,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO recurring_audit(company_id,rule_id,revision,actor_id,action,snapshot) VALUES($1,$2,$3,$4,$5,$6)").bind(c).bind(id(&v.id)?).bind(v.revision).bind(a).bind(action).bind(serde_json::to_value(v).map_err(err)?).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: &serde_json::Value,
) -> Result<Option<Rule>> {
    if let Some(r) = sqlx::query(
        "SELECT * FROM recurring_command WHERE company_id=$1 AND request_id=$2::text::uuid",
    )
    .bind(c)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    {
        if r.get::<i64, _>("actor_id") != a || r.get::<serde_json::Value, _>("command") != *input {
            return Err(Error::Conflict);
        }
        return Ok(Some(load(tx, c, r.get("rule_id")).await?));
    }
    Ok(None)
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: serde_json::Value,
    v: &Rule,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO recurring_command(company_id,request_id,actor_id,rule_id,command) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(key).bind(a).bind(id(&v.id)?).bind(input).execute(&mut **tx).await.map_err(err)?;
    audit(tx, a, c, v, action).await
}
async fn check(tx: &mut Transaction<'_, Postgres>, c: i64, data: &Data) -> Result<()> {
    check_contact(tx, c, data.invoice.contact_id.as_deref()).await?;
    if let Some(m) = &data.mail {
        m.recipient
            .parse::<lettre::Address>()
            .map_err(|_| Error::Invalid(vec!["mail".into()]))?;
    }
    Ok(())
}
#[async_trait]
impl RecurringStore for PgRecurring {
    async fn preview(&self, a: &str, c: &str, cmd: Preview) -> Result<Vec<chrono::NaiveDate>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        authorize(&mut tx, a, c, false).await?;
        if !cmd.schedule.valid() || cmd.index > 10000 {
            return Err(Error::Invalid(vec!["schedule".into()]));
        }
        Ok(cmd.schedule.preview(cmd.index))
    }

    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Rule> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, a, c).await?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(v) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        check(&mut tx, c, &cmd.data).await?;
        let i:i64=sqlx::query_scalar("INSERT INTO recurring_rule(company_id,actor_id,data,next_on) VALUES($1,$2,$3,$4) RETURNING id").bind(c).bind(a).bind(serde_json::to_value(&cmd.data).map_err(err)?).bind(cmd.data.schedule.date(0)).fetch_one(&mut *tx).await.map_err(err)?;
        let v = load(&mut tx, c, i).await?;
        record(&mut tx, a, c, &cmd.request_id, input, &v, "create").await?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
    async fn command(&self, a: &str, c: &str, i: &str, cmd: Command) -> Result<Rule> {
        self.command_with_rate_provider(a, c, i, cmd, &crate::exchange_rate::Cnb::default())
            .await
    }
    async fn get(&self, a: &str, c: &str, i: &str) -> Result<Rule> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c, id(i)?).await
    }
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let page = q.page.unwrap_or(1);
        let predicate="company_id=$1 AND strpos(lower(concat_ws(' ',data->>'name',data#>>'{invoice,customer,name}')),lower($2))>0";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM recurring_rule WHERE {predicate}"
        ))
        .bind(c)
        .bind(&q.query)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let ids: Vec<i64> = sqlx::query_scalar(&format!(
            "SELECT id FROM recurring_rule WHERE {predicate} ORDER BY id DESC LIMIT 25 OFFSET $3"
        ))
        .bind(c)
        .bind(&q.query)
        .bind((page - 1) * 25)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
        let mut items = vec![];
        for i in ids {
            items.push(load(&mut tx, c, i).await?);
        }
        Ok(Page { items, total, page })
    }
}
impl PgRecurring {
    /// At most one due occurrence per tick. No network I/O while holding transaction locks.
    pub async fn process_one(&self) -> Result<bool> {
        let row=sqlx::query("SELECT r.id,r.company_id,r.actor_id,r.revision FROM recurring_rule r JOIN company c ON c.id=r.company_id WHERE c.deleted_at IS NULL AND r.status='active' AND r.next_on<=(now() AT TIME ZONE (r.data#>>'{schedule,timezone}'))::date ORDER BY r.next_on,r.id LIMIT 1").fetch_optional(&self.0).await.map_err(err)?;
        let Some(r) = row else { return Ok(false) };
        let a = r.get::<i64, _>("actor_id");
        let c = r.get::<i64, _>("company_id");
        let i = r.get::<i64, _>("id");
        let revision = r.get("revision");
        let result = self
            .command(
                &a.to_string(),
                &c.to_string(),
                &i.to_string(),
                Command {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    revision,
                    action: Action::Run,
                },
            )
            .await;
        if let Err(e) = result {
            let mut tx = self.0.begin().await.map_err(err)?;
            sqlx::query("SELECT id FROM company WHERE id=$1 FOR UPDATE")
                .bind(c)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            let changed=sqlx::query("UPDATE recurring_rule SET status='error',error_code=$4,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 AND revision=$3 AND status='active'").bind(c).bind(i).bind(revision).bind(error_code(&e)).execute(&mut *tx).await.map_err(err)?.rows_affected();
            if changed > 0 {
                let v = load(&mut tx, c, i).await?;
                audit(&mut tx, a, c, &v, "error").await?;
            }
            tx.commit().await.map_err(err)?;
        }
        Ok(true)
    }
    pub async fn enqueue_one(
        &self,
        store: &crate::invoice_delivery::PgInvoiceDelivery,
    ) -> Result<bool> {
        let row=sqlx::query("UPDATE recurring_occurrence SET next_attempt=now()+interval '2 minutes' WHERE (company_id,rule_id,index)=(SELECT o.company_id,o.rule_id,o.index FROM recurring_occurrence o JOIN company c ON c.id=o.company_id WHERE o.mail_command IS NOT NULL AND o.delivery_id IS NULL AND o.next_attempt<=now() AND c.deleted_at IS NULL ORDER BY o.created_at FOR UPDATE OF o SKIP LOCKED LIMIT 1) RETURNING *").fetch_optional(&self.0).await.map_err(err)?;
        let Some(r) = row else { return Ok(false) };
        let c = r.get::<i64, _>("company_id");
        let i = r.get::<i64, _>("rule_id");
        let n = r.get::<i32, _>("index");
        let result = faktura_application::delivery::send(
            store,
            &r.get::<i64, _>("actor_id").to_string(),
            &c.to_string(),
            &r.get::<i64, _>("invoice_id").to_string(),
            serde_json::from_value(r.get("mail_command")).map_err(err)?,
        )
        .await;
        let (delivery, error) = match result {
            Ok(v) => (Some(id(&v.id)?), None),
            Err(e) => (None, Some(error_code(&e))),
        };
        sqlx::query("UPDATE recurring_occurrence SET delivery_id=$4,error_code=$5,next_attempt=now()+interval '5 minutes' WHERE company_id=$1 AND rule_id=$2 AND index=$3").bind(c).bind(i).bind(n).bind(delivery).bind(error).execute(&self.0).await.map_err(err)?;
        Ok(true)
    }
}

impl PgRecurring {
    pub async fn command_with_rate_provider(
        &self,
        a: &str,
        c: &str,
        i: &str,
        cmd: Command,
        provider: &dyn faktura_application::exchange_rate::RateProvider,
    ) -> Result<Rule> {
        let mut proposal = None;
        if matches!(cmd.action, Action::Run) {
            // Preflight authorization and replay before network; drop all locks before I/O.
            let mut tx = self.0.begin().await.map_err(err)?;
            let (_, company) = authorize(&mut tx, a, c, true).await?;
            let rule = load(&mut tx, company, id(i)?).await?;
            let seen:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM recurring_command WHERE company_id=$1 AND request_id=$2::text::uuid)").bind(company).bind(&cmd.request_id).fetch_one(&mut *tx).await.map_err(err)?;
            if !seen && rule.data.rate_policy == RatePolicy::CnbIssueDate {
                if rule.revision != cmd.revision || rule.status != "active" {
                    return Err(Error::Conflict);
                }
                let on = rule
                    .data
                    .schedule
                    .date(rule.generated as u32)
                    .ok_or(Error::Conflict)?;
                tx.commit().await.map_err(err)?;
                proposal = Some(
                    provider
                        .lookup(&rule.data.invoice.calculation.currency, on)
                        .await?,
                );
            } else {
                tx.commit().await.map_err(err)?;
            }
        }
        // The shared command rechecks authorization, replay, revision and schedule after I/O.
        self.command_prepared(a, c, i, cmd, proposal).await
    }
    async fn command_prepared(
        &self,
        a: &str,
        c: &str,
        i: &str,
        cmd: Command,
        proposal: Option<faktura_application::exchange_rate::Rate>,
    ) -> Result<Rule> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, a, c).await?;
        let i = id(i)?;
        let input = serde_json::json!({"id":i.to_string(),"command":cmd});
        if let Some(v) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        let v = load(&mut tx, c, i).await?;
        if v.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        let action = match cmd.action {
            Action::Save { data } => {
                if v.status == "active"
                    || (v.generated > 0 && !v.data.schedule.same_anchor(&data.schedule))
                {
                    return Err(Error::Conflict);
                }
                check(&mut tx, c, &data).await?;
                sqlx::query("UPDATE recurring_rule SET data=$3,next_on=$4,status='paused',error_code=NULL WHERE company_id=$1 AND id=$2").bind(c).bind(i).bind(serde_json::to_value(&data).map_err(err)?).bind(data.schedule.date(v.generated as u32)).execute(&mut *tx).await.map_err(err)?;
                "save"
            }
            Action::Activate => {
                let on = v
                    .data
                    .schedule
                    .date(v.generated as u32)
                    .ok_or(Error::Conflict)?;
                materialize(&v.data, on)?;
                sqlx::query("UPDATE recurring_rule SET status='active',error_code=NULL,actor_id=$3 WHERE company_id=$1 AND id=$2").bind(c).bind(i).bind(a).execute(&mut *tx).await.map_err(err)?;
                "activate"
            }
            Action::Pause => {
                sqlx::query("UPDATE recurring_rule SET status='paused',error_code=NULL WHERE company_id=$1 AND id=$2").bind(c).bind(i).execute(&mut *tx).await.map_err(err)?;
                "pause"
            }
            Action::Run => {
                if v.status != "active" {
                    return Err(Error::Conflict);
                }
                let on = v
                    .data
                    .schedule
                    .date(v.generated as u32)
                    .ok_or(Error::Conflict)?;
                let today: chrono::NaiveDate =
                    sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date")
                        .bind(&v.data.schedule.timezone)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(err)?;
                if on > today {
                    return Err(Error::Conflict);
                }
                let mut invoice = materialize(&v.data, on)?;
                if v.data.rate_policy == RatePolicy::CnbIssueDate {
                    let rate = proposal.as_ref().ok_or(Error::Unavailable)?;
                    if rate.source != "CNB"
                        || rate.currency != invoice.calculation.currency
                        || rate.requested_on != on
                        || !faktura_domain::exchange::validate(
                            &rate.currency,
                            Some(&rate.exchange_rate),
                        )
                    {
                        return Err(Error::Unavailable);
                    }
                    invoice.calculation.exchange_rate = Some(rate.exchange_rate.clone());
                }
                let mut draft = create_in_tx(
                    &mut tx,
                    &a.to_string(),
                    &c.to_string(),
                    CreateDraft {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        data: invoice,
                    },
                )
                .await?;
                if v.data.auto_issue {
                    draft = issue_in_tx(
                        &mut tx,
                        &a.to_string(),
                        &c.to_string(),
                        &draft.id,
                        Issue {
                            request_id: uuid::Uuid::new_v4().to_string(),
                            revision: draft.revision,
                        },
                    )
                    .await?;
                }
                let request = uuid::Uuid::new_v4().to_string();
                let mail = v.data.mail.as_ref().map(|m| SendQuote {
                    purpose: None,
                    settlement_revision: None,
                    automation_revision: None,
                    request_id: request.clone(),
                    revision: draft.revision,
                    recipient: m.recipient.clone(),
                    subject: m.subject.clone(),
                    body: m.body.clone(),
                    attachment_ids: vec![],
                    share_id: None,
                });
                let mut occurrence_snapshot = serde_json::to_value(&v.data).map_err(err)?;
                if let Some(rate) = proposal.as_ref() {
                    occurrence_snapshot["invoice"]["calculation"]["exchange_rate"] =
                        serde_json::to_value(&rate.exchange_rate).map_err(err)?;
                    occurrence_snapshot["rate_observation"] =
                        serde_json::to_value(rate).map_err(err)?;
                }
                sqlx::query("INSERT INTO recurring_occurrence(company_id,rule_id,index,scheduled_on,invoice_id,snapshot,actor_id,mail_request,mail_command) VALUES($1,$2,$3,$4,$5,$6,$7,$8::text::uuid,$9)").bind(c).bind(i).bind(v.generated).bind(on).bind(id(&draft.id)?).bind(occurrence_snapshot).bind(a).bind(request).bind(mail.map(serde_json::to_value).transpose().map_err(err)?).execute(&mut *tx).await.map_err(err)?;
                let next = v.data.schedule.date(v.generated as u32 + 1);
                sqlx::query("UPDATE recurring_rule SET generated=generated+1,next_on=$3,status=$4,error_code=NULL WHERE company_id=$1 AND id=$2").bind(c).bind(i).bind(next).bind(if next.is_some(){"active"}else{"completed"}).execute(&mut *tx).await.map_err(err)?;
                "run"
            }
        };
        sqlx::query("UPDATE recurring_rule SET revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(c).bind(i).execute(&mut *tx).await.map_err(err)?;
        let v = load(&mut tx, c, i).await?;
        record(&mut tx, a, c, &cmd.request_id, input, &v, action).await?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
}

use crate::draft::{authorize, map as invoice_map};
use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    credit_note::*,
    draft::Draft,
};
use faktura_domain::{
    credit_note::{calculate, capacities, decimal, money},
    invoice::Totals,
};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::BTreeMap;
#[derive(Clone)]
pub struct PgCredit(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
pub(crate) async fn load(tx: &mut Transaction<'_, Postgres>, c: i64, i: i64) -> Result<Document> {
    let r = sqlx::query("SELECT * FROM credit_note WHERE company_id=$1 AND id=$2")
        .bind(c)
        .bind(i)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    let source: Draft = serde_json::from_value(r.get("source")).map_err(err)?;
    let totals: Totals = serde_json::from_value(r.get("totals")).map_err(err)?;
    let events:Vec<Event>=sqlx::query("SELECT *,amount::text AS amount_text,date::text AS date_text,created_at::text AS time_text FROM credit_note_event WHERE company_id=$1 AND credit_id=$2 ORDER BY id").bind(c).bind(i).fetch_all(&mut **tx).await.map_err(err)?.into_iter().map(|e|Event{id:e.get::<i64,_>("id").to_string(),kind:e.get("kind"),amount:e.get("amount_text"),date:e.get("date_text"),method:e.get("method"),reason:e.get("reason"),reverses:e.get::<Option<i64>,_>("reverses").map(|v|v.to_string()),actor_id:e.get::<i64,_>("actor_id").to_string(),created_at:e.get("time_text")}).collect();
    let sum = |kind: &str| {
        events
            .iter()
            .filter(|e| {
                e.kind == kind && !events.iter().any(|r| r.reverses.as_ref() == Some(&e.id))
            })
            .fold(Decimal::ZERO, |s, e| s + decimal(&e.amount))
    };
    let offset = sum("offset");
    let refunded = sum("refund");
    Ok(Document {
        id: i.to_string(),
        company_id: c.to_string(),
        invoice_id: source.id.clone(),
        invoice_number: source.number.clone().ok_or(Error::Conflict)?,
        number: r.get("number"),
        status: r.get("status"),
        revision: r.get("revision"),
        source,
        data: serde_json::from_value(r.get("data")).map_err(err)?,
        remaining: money(-decimal(&totals.payable) - offset - refunded),
        totals,
        offset: money(offset),
        refunded: money(refunded),
        events,
    })
}
async fn used(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    invoice: i64,
) -> Result<BTreeMap<String, Decimal>> {
    let values: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT data FROM credit_note WHERE company_id=$1 AND invoice_id=$2 AND status='issued'",
    )
    .bind(c)
    .bind(invoice)
    .fetch_all(&mut **tx)
    .await
    .map_err(err)?;
    let mut result = BTreeMap::new();
    for value in values {
        let data: Data = serde_json::from_value(value).map_err(err)?;
        for line in data.lines {
            *result.entry(line.id).or_default() += decimal(&line.amount)
        }
    }
    Ok(result)
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: serde_json::Value,
    v: &Document,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO credit_note_command(company_id,request_id,actor_id,command,credit_id) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(key).bind(a).bind(input).bind(id(&v.id)?).execute(&mut **tx).await.map_err(err)?;
    sqlx::query("INSERT INTO credit_note_audit(company_id,credit_id,revision,actor_id,action,snapshot) VALUES($1,$2,$3,$4,$5,$6)").bind(c).bind(id(&v.id)?).bind(v.revision).bind(a).bind(action).bind(serde_json::to_value(v).map_err(err)?).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: &serde_json::Value,
) -> Result<Option<Document>> {
    if let Some(r) = sqlx::query(
        "SELECT * FROM credit_note_command WHERE company_id=$1 AND request_id=$2::text::uuid",
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
        return Ok(Some(load(tx, c, r.get("credit_id")).await?));
    }
    Ok(None)
}
async fn today(tx: &mut Transaction<'_, Postgres>) -> Result<chrono::NaiveDate> {
    sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
        .fetch_one(&mut **tx)
        .await
        .map_err(err)
}
#[async_trait]
impl CreditStore for PgCredit {
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::json!({"operation":"create","command":cmd});
        if let Some(v) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        let invoice = id(&cmd.invoice_id)?;
        let source = invoice_map(
            sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(c)
                .bind(invoice)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
        )?;
        if source.number.is_none() {
            return Err(Error::Conflict);
        }
        let original = source.totals.as_ref().ok_or(Error::Conflict)?;
        let consumed = used(&mut tx, c, invoice).await?;
        let lines = capacities(original)
            .into_iter()
            .map(|mut l| {
                l.amount =
                    money(decimal(&l.amount) - consumed.get(&l.id).copied().unwrap_or_default());
                l
            })
            .filter(|l| decimal(&l.amount) > Decimal::ZERO)
            .collect::<Vec<_>>();
        let totals =
            calculate(&source.data, original, &lines, &consumed).map_err(Error::Invalid)?;
        let date = today(&mut tx).await?;
        let data = Data {
            reason: String::new(),
            issued_on: date,
            tax_on: date,
            due_on: date,
            language: source.data.language.clone(),
            lines,
        };
        let i:i64=sqlx::query_scalar("INSERT INTO credit_note(company_id,invoice_id,source,data,totals) VALUES($1,$2,$3,$4,$5) RETURNING id").bind(c).bind(invoice).bind(serde_json::to_value(source).map_err(err)?).bind(serde_json::to_value(data).map_err(err)?).bind(serde_json::to_value(totals).map_err(err)?).fetch_one(&mut *tx).await.map_err(err)?;
        let v = load(&mut tx, c, i).await?;
        record(&mut tx, a, c, &cmd.request_id, input, &v, "create").await?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
    async fn command(&self, a: &str, c: &str, i: &str, cmd: Command) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let result = command_in_tx(&mut tx, a, c, i, cmd).await?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn get(&self, a: &str, c: &str, i: &str) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c, id(i)?).await
    }
    async fn pdf(&self, a: &str, c: &str, i: &str) -> Result<Vec<u8>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT pdf FROM credit_note WHERE company_id=$1 AND id=$2 AND status='issued'",
        )
        .bind(c)
        .bind(id(i)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .flatten()
        .ok_or(Error::NotFound)
    }
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let page = q.page.unwrap_or(1);
        let invoice = q.invoice_id.as_deref().map(id).transpose()?;
        let predicate="company_id=$1 AND ($2::bigint IS NULL OR invoice_id=$2) AND strpos(lower(concat_ws(' ',number,source->>'number',source#>>'{data,customer,name}',data->>'reason')),lower($3))>0";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM credit_note WHERE {predicate}"
        ))
        .bind(c)
        .bind(invoice)
        .bind(&q.query)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let ids: Vec<i64> = sqlx::query_scalar(&format!(
            "SELECT id FROM credit_note WHERE {predicate} ORDER BY id DESC LIMIT 25 OFFSET $4"
        ))
        .bind(c)
        .bind(invoice)
        .bind(&q.query)
        .bind((page - 1) * 25)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
        let mut items = Vec::new();
        for i in ids {
            items.push(load(&mut tx, c, i).await?)
        }
        Ok(Page { items, total, page })
    }
}

pub(crate) async fn command_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    i: &str,
    cmd: Command,
) -> Result<Document> {
    let (a, c) = authorize(tx, a, c, true).await?;
    let i = id(i)?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    let input = serde_json::json!({"operation":"command","id":i.to_string(),"command":cmd});
    if let Some(v) = replay(tx, a, c, &cmd.request_id, &input).await? {
        return Ok(v);
    }
    let invoice: i64 =
        sqlx::query_scalar("SELECT invoice_id FROM credit_note WHERE company_id=$1 AND id=$2")
            .bind(c)
            .bind(i)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
    sqlx::query("SELECT id FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE")
        .bind(c)
        .bind(invoice)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    sqlx::query("SELECT id FROM credit_note WHERE company_id=$1 AND id=$2 FOR UPDATE")
        .bind(c)
        .bind(i)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    let mut v = load(tx, c, i).await?;
    if v.revision != cmd.revision {
        return Err(Error::Conflict);
    }
    let date = today(tx).await?;
    let action = match &cmd.action {
        Action::Update { data } => {
            if v.status != "draft" {
                return Err(Error::Conflict);
            }
            let totals = calculate(
                &v.source.data,
                v.source.totals.as_ref().ok_or(Error::Conflict)?,
                &data.lines,
                &used(tx, c, invoice).await?,
            )
            .map_err(Error::Invalid)?;
            sqlx::query("UPDATE credit_note SET data=$3,totals=$4 WHERE company_id=$1 AND id=$2")
                .bind(c)
                .bind(i)
                .bind(serde_json::to_value(data).map_err(err)?)
                .bind(serde_json::to_value(totals).map_err(err)?)
                .execute(&mut **tx)
                .await
                .map_err(err)?;
            "update"
        }
        Action::Issue => {
            if v.status != "draft" {
                return Err(Error::Conflict);
            }
            if v.data.reason.trim().is_empty()
                || v.data.issued_on > date
                || Some(v.data.issued_on) < v.source.data.issued_on
                || v.data.tax_on > v.data.issued_on
                || Some(v.data.tax_on) < v.source.data.tax_on.or(v.source.data.issued_on)
                || v.data.due_on < v.data.issued_on
            {
                return Err(Error::Invalid(vec!["credit_dates_reason".into()]));
            }
            v.totals = calculate(
                &v.source.data,
                v.source.totals.as_ref().ok_or(Error::Conflict)?,
                &v.data.lines,
                &used(tx, c, invoice).await?,
            )
            .map_err(Error::Invalid)?;
            crate::stock_documents::credit(tx, a, c, i, invoice, &v.data).await?;
            let year = v.data.issued_on.format("%Y").to_string();
            let n:i64=sqlx::query_scalar("INSERT INTO credit_note_sequence(company_id,year,next_value) VALUES($1,$2::text::integer,2) ON CONFLICT(company_id,year) DO UPDATE SET next_value=credit_note_sequence.next_value+1 RETURNING next_value-1").bind(c).bind(&year).fetch_one(&mut **tx).await.map_err(err)?;
            v.number = Some(format!("DB{year}{n:04}"));
            static RENDERERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
            let permit = RENDERERS.acquire().await.map_err(err)?;
            let pdf_doc = v.clone();
            let pdf = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                crate::invoice_pdf::render_credit(&pdf_doc)
            })
            .await
            .map_err(err)?
            .map_err(err)?;
            sqlx::query("UPDATE credit_note SET number=$3,status='issued',totals=$4,pdf=$5 WHERE company_id=$1 AND id=$2").bind(c).bind(i).bind(&v.number).bind(serde_json::to_value(&v.totals).map_err(err)?).bind(pdf).execute(&mut **tx).await.map_err(err)?;
            "issue"
        }
        Action::Cancel => {
            if v.status != "draft" {
                return Err(Error::Conflict);
            }
            sqlx::query("UPDATE credit_note SET status='cancelled' WHERE company_id=$1 AND id=$2")
                .bind(c)
                .bind(i)
                .execute(&mut **tx)
                .await
                .map_err(err)?;
            "cancel"
        }
        Action::Offset | Action::Refund { .. } | Action::Reverse { .. } => {
            if v.status != "issued" {
                return Err(Error::Conflict);
            }
            let (kind, amount, on, method, reason, reverses) = match &cmd.action {
                Action::Offset => {
                    let outstanding:String=sqlx::query_scalar("SELECT outstanding::text FROM invoice_settlement WHERE company_id=$1 AND invoice_id=$2").bind(c).bind(invoice).fetch_one(&mut **tx).await.map_err(err)?;
                    if decimal(&v.remaining) <= Decimal::ZERO
                        || decimal(&v.remaining) > decimal(&outstanding)
                    {
                        return Err(Error::Invalid(vec!["credit_offset".into()]));
                    }
                    (
                        "offset",
                        v.remaining.clone(),
                        date,
                        String::new(),
                        String::new(),
                        None,
                    )
                }
                Action::Refund {
                    amount,
                    date: on,
                    method,
                    reason,
                } => {
                    if (v.totals.currency != "CZK" && method != "bank_transfer")
                        || decimal(amount) > decimal(&v.remaining)
                        || *on < v.data.issued_on
                        || *on > date
                    {
                        return Err(Error::Invalid(vec!["credit_refund".into()]));
                    }
                    (
                        "refund",
                        amount.clone(),
                        *on,
                        method.clone(),
                        reason.clone(),
                        None,
                    )
                }
                Action::Reverse { event_id, reason } => {
                    if crate::treasury_documents::linked(tx, c, "credit", i, event_id).await? {
                        return Err(Error::Conflict);
                    }
                    let old = v
                        .events
                        .iter()
                        .find(|e| &e.id == event_id)
                        .ok_or(Error::NotFound)?;
                    if old.kind == "reversal"
                        || v.events
                            .iter()
                            .any(|e| e.reverses.as_ref() == Some(event_id))
                    {
                        return Err(Error::Conflict);
                    }
                    (
                        "reversal",
                        old.amount.clone(),
                        date,
                        String::new(),
                        reason.clone(),
                        Some(id(event_id)?),
                    )
                }
                _ => unreachable!(),
            };
            sqlx::query("INSERT INTO credit_note_event(company_id,credit_id,invoice_id,kind,amount,date,method,reason,reverses,actor_id) VALUES($1,$2,$3,$4,$5::text::numeric,$6,$7,$8,$9,$10)").bind(c).bind(i).bind(invoice).bind(kind).bind(amount).bind(on).bind(method).bind(reason).bind(reverses).bind(a).execute(&mut **tx).await.map_err(err)?;
            kind
        }
    };
    sqlx::query(
        "UPDATE credit_note SET revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2",
    )
    .bind(c)
    .bind(i)
    .execute(&mut **tx)
    .await
    .map_err(err)?;
    let result = load(tx, c, i).await?;
    record(tx, a, c, &cmd.request_id, input, &result, action).await?;
    Ok(result)
}

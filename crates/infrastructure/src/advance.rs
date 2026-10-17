use crate::draft::authorize;
use async_trait::async_trait;
use faktura_application::{
    advance::*,
    company::{CompanyError as Error, Result},
};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgAdvance(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
async fn document(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    d: &str,
    lock: bool,
) -> Result<sqlx::postgres::PgRow> {
    sqlx::query(&format!(
        "SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 AND kind='advance' {}",
        if lock { "FOR UPDATE" } else { "" }
    ))
    .bind(c)
    .bind(id(d)?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    .ok_or(Error::NotFound)
}
#[async_trait]
impl AdvanceStore for PgAdvance {
    async fn pdf(&self, a: &str, c: &str, d: &str) -> Result<Vec<u8>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let r = document(&mut tx, c, d, false).await?;
        r.get::<Option<Vec<u8>>, _>("advance_pdf")
            .ok_or(Error::Unavailable)
    }
    async fn payments(&self, a: &str, c: &str, d: &str) -> Result<Vec<Receipt>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        document(&mut tx, c, d, false).await?;
        let rows=sqlx::query("SELECT p.id,p.amount::text,p.paid_on::text,p.method,p.note,p.reversed_at IS NOT NULL AS reversed,p.reason,t.id::text AS tax_receipt_id,t.number AS tax_receipt_number FROM advance_payment p LEFT JOIN tax_receipt t ON t.company_id=p.company_id AND t.payment_id=p.id WHERE p.company_id=$1 AND p.document_id=$2 ORDER BY p.id DESC").bind(c).bind(id(d)?).fetch_all(&mut *tx).await.map_err(err)?;
        Ok(rows
            .into_iter()
            .map(|r| Receipt {
                id: r.get::<i64, _>("id").to_string(),
                amount: r.get("amount"),
                paid_on: r.get("paid_on"),
                method: r.get("method"),
                note: r.get("note"),
                reversed: r.get("reversed"),
                reason: r.get("reason"),
                tax_receipt_id: r.get("tax_receipt_id"),
                tax_receipt_number: r.get("tax_receipt_number"),
            })
            .collect())
    }
    async fn register(&self, a: &str, c: &str, d: &str, cmd: Register) -> Result<()> {
        let mut tx = self.0.begin().await.map_err(err)?;
        register_in_tx(&mut tx, a, c, d, cmd).await?;
        tx.commit().await.map_err(err)
    }
    async fn reverse(&self, a: &str, c: &str, d: &str, p: &str, cmd: Reverse) -> Result<()> {
        let mut tx = self.0.begin().await.map_err(err)?;
        reverse_in_tx(&mut tx, a, c, d, p, cmd).await?;
        tx.commit().await.map_err(err)
    }
}
async fn update(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    d: &str,
    amount: &str,
    action: &str,
) -> Result<()> {
    let r=sqlx::query("UPDATE sales_document SET advance_paid=advance_paid+$3::text::numeric,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(d)?).bind(amount).fetch_one(&mut **tx).await.map_err(err)?;
    crate::sales::audit(tx, a, &crate::sales::map(r)?, action).await
}

pub(crate) async fn register_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    d: &str,
    cmd: Register,
) -> Result<()> {
    let (a, c) = authorize(tx, a, c, true).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    let r = document(tx, c, d, true).await?;
    let command = serde_json::to_value(&cmd).map_err(err)?;
    if let Some(old) = sqlx::query(
        "SELECT * FROM advance_payment WHERE company_id=$1 AND request_id=$2::text::uuid",
    )
    .bind(c)
    .bind(&cmd.request_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    {
        return if old.get::<i64, _>("actor_id") == a
            && old.get::<i64, _>("document_id") == id(d)?
            && old.get::<serde_json::Value, _>("command") == command
        {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    crate::advance_settlement::ensure_free(tx, c, id(d)?).await?;
    if r.get::<bool, _>("archived") || r.get::<i64, _>("revision") != cmd.revision {
        return Err(Error::Conflict);
    }
    let data: faktura_application::sales::Data =
        serde_json::from_value(r.get("data")).map_err(err)?;
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    if Some(cmd.payment.paid_on) < data.invoice.issued_on || cmd.payment.paid_on > today {
        return Err(Error::Invalid(vec!["paid_on".into()]));
    }
    sqlx::query("INSERT INTO advance_payment(company_id,document_id,actor_id,request_id,command,amount,paid_on,method,note) VALUES($1,$2,$3,$4::text::uuid,$5,$6::text::numeric,$7,$8,$9)").bind(c).bind(id(d)?).bind(a).bind(&cmd.request_id).bind(command).bind(&cmd.payment.amount).bind(cmd.payment.paid_on).bind(&cmd.payment.method).bind(&cmd.payment.note).execute(&mut **tx).await.map_err(err)?;
    update(tx, a, c, d, &cmd.payment.amount, "payment").await?;
    Ok(())
}

pub(crate) async fn reverse_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    d: &str,
    p: &str,
    cmd: Reverse,
) -> Result<()> {
    let (a, c) = authorize(tx, a, c, true).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    let r = document(tx, c, d, true).await?;
    if crate::treasury_documents::linked(tx, c, "advance", id(d)?, p).await? {
        return Err(Error::Conflict);
    }
    let payment=sqlx::query("SELECT *,amount::text AS amount_text,reversed_at IS NOT NULL AS reversed FROM advance_payment WHERE company_id=$1 AND document_id=$2 AND id=$3 FOR UPDATE").bind(c).bind(id(d)?).bind(id(p)?).fetch_optional(&mut **tx).await.map_err(err)?.ok_or(Error::NotFound)?;
    if payment.get::<bool, _>("reversed") {
        return if payment.get::<Option<i64>, _>("reversed_by") == Some(a)
            && payment.get::<Option<String>, _>("reason") == Some(cmd.reason)
        {
            Ok(())
        } else {
            Err(Error::Conflict)
        };
    }
    if r.get::<i64, _>("revision") != cmd.revision || r.get::<bool, _>("archived") {
        return Err(Error::Conflict);
    }
    crate::advance_settlement::ensure_free(tx, c, id(d)?).await?;
    let has_tax: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tax_receipt WHERE company_id=$1 AND advance_id=$2 AND payment_id=$3)").bind(c).bind(id(d)?).bind(id(p)?).fetch_one(&mut **tx).await.map_err(err)?;
    if has_tax {
        return Err(Error::Conflict);
    }
    sqlx::query("UPDATE advance_payment SET reversed_at=now(),reversed_by=$4,reason=$5 WHERE company_id=$1 AND document_id=$2 AND id=$3").bind(c).bind(id(d)?).bind(id(p)?).bind(a).bind(cmd.reason).execute(&mut **tx).await.map_err(err)?;
    update(
        tx,
        a,
        c,
        d,
        &format!("-{}", payment.get::<String, _>("amount_text")),
        "payment_reverse",
    )
    .await?;
    Ok(())
}

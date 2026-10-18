use async_trait::async_trait;
use chrono::Datelike;
use faktura_application::received::*;
use faktura_domain::received::{amount, money, totals, Kind};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgReceived(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(s: &str) -> Result<i64> {
    s.parse().map_err(|_| Error::NotFound)
}
fn map(r: sqlx::postgres::PgRow) -> Result<Document> {
    let data: Data = serde_json::from_value(r.get("data")).map_err(err)?;
    let t = totals(&data).map_err(Error::Invalid)?;
    let paid: Decimal = r.get("paid");
    let total: Decimal = r.get("total");
    Ok(Document {
        id: r.get::<i64, _>("id").to_string(),
        company_id: r.get::<i64, _>("company_id").to_string(),
        number: r.get("number"),
        revision: r.get("revision"),
        cancelled: r.get("cancelled"),
        data,
        totals: t,
        paid: money(paid),
        remaining: money(if r.get::<bool, _>("cancelled") {
            Decimal::ZERO
        } else {
            total - paid
        }),
        payment_status: if paid == total {
            "paid"
        } else if paid > Decimal::ZERO {
            "partial"
        } else {
            "unpaid"
        }
        .into(),
        payments: serde_json::from_value(r.get("payments")).map_err(err)?,
    })
}
pub(crate) async fn load(tx: &mut Transaction<'_, Postgres>, c: i64, d: i64) -> Result<Document> {
    map(
        sqlx::query("SELECT * FROM received_invoice WHERE company_id=$1 AND id=$2 FOR UPDATE")
            .bind(c)
            .bind(d)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?,
    )
}
async fn lock(tx: &mut Transaction<'_, Postgres>, a: &str, c: &str) -> Result<(i64, i64)> {
    let (a, c) = crate::draft::authorize(tx, a, c, true).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok((a, c))
}
async fn contact(tx: &mut Transaction<'_, Postgres>, c: i64, data: &Data) -> Result<()> {
    if let Some(i) = &data.supplier_contact_id {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM contact WHERE company_id=$1 AND id=$2 AND NOT archived FOR SHARE",
        )
        .bind(c)
        .bind(id(i)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    }
    Ok(())
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: &serde_json::Value,
) -> Result<Option<Document>> {
    let r = sqlx::query(
        "SELECT * FROM received_command WHERE company_id=$1 AND request_id=$2::text::uuid",
    )
    .bind(c)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?;
    r.map(|r| {
        if r.get::<i64, _>("actor_id") != a || r.get::<serde_json::Value, _>("input") != *input {
            return Err(Error::Conflict);
        }
        serde_json::from_value(r.get("result")).map_err(err)
    })
    .transpose()
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: serde_json::Value,
    d: &Document,
    action: &str,
) -> Result<()> {
    let result = serde_json::to_value(d).map_err(err)?;
    sqlx::query("INSERT INTO received_command(company_id,request_id,actor_id,invoice_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5,$6)").bind(c).bind(key).bind(a).bind(id(&d.id)?).bind(input).bind(&result).execute(&mut **tx).await.map_err(err)?;
    sqlx::query("INSERT INTO received_audit(company_id,invoice_id,actor_id,revision,action,snapshot) VALUES($1,$2,$3,$4,$5,$6)").bind(c).bind(id(&d.id)?).bind(a).bind(d.revision).bind(action).bind(result).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
#[async_trait]
impl ReceivedStore for PgReceived {
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, a, c).await?;
        let input = serde_json::json!({"create":&cmd});
        if let Some(d) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(d);
        }
        let doc = create_in_tx(&mut tx, a, c, &cmd, input).await?;
        tx.commit().await.map_err(err)?;
        Ok(doc)
    }
    async fn command(&self, a: &str, c: &str, d: &str, cmd: Command) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let value = command_in_tx(&mut tx, a, c, d, cmd).await?;
        tx.commit().await.map_err(err)?;
        Ok(value)
    }
    async fn get(&self, a: &str, c: &str, d: &str) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c, id(d)?).await
    }
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        let page = q.page.unwrap_or(1);
        if !(1..=10000).contains(&page)
            || q.query.len() > 200
            || !["", "all", "unpaid", "paid", "overdue", "cancelled"].contains(&q.status.as_str())
        {
            return Err(Error::Invalid(vec!["query".into()]));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let predicate="company_id=$1 AND kind=$4 AND ($2='' OR position(lower($2) in lower(number||' '||(data->>'external_number')||' '||(data->'supplier'->>'name')||' '||(data->>'description')))>0) AND CASE $3 WHEN 'cancelled' THEN cancelled WHEN 'paid' THEN NOT cancelled AND paid=total WHEN 'unpaid' THEN NOT cancelled AND paid<total WHEN 'overdue' THEN NOT cancelled AND paid<total AND due_on<(now() AT TIME ZONE 'Europe/Prague')::date ELSE NOT cancelled END";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM received_invoice WHERE {predicate}"
        ))
        .bind(c)
        .bind(&q.query)
        .bind(&q.status)
        .bind(q.kind.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items=sqlx::query(&format!("SELECT * FROM received_invoice WHERE {predicate} ORDER BY received_on DESC,id DESC LIMIT 25 OFFSET $5")).bind(c).bind(q.query).bind(q.status).bind(q.kind.as_str()).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(map).collect::<Result<Vec<_>>>()?;
        Ok(Page { items, total, page })
    }
    async fn preview(&self, a: &str, c: &str, data: Data) -> Result<Totals> {
        let mut tx = self.0.begin().await.map_err(err)?;
        crate::draft::authorize(&mut tx, a, c, false).await?;
        totals(&data).map_err(Error::Invalid)
    }
}

fn receipt_payment(key: &str, actor: i64, data: &Data, total: &str) -> Payment {
    Payment {
        id: key.into(),
        input: faktura_domain::payment::PaymentInput {
            amount: total.into(),
            paid_on: data.issued_on,
            method: data.receipt_method.clone().unwrap_or_default(),
            note: String::new(),
        },
        actor_id: actor.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        reversed_at: None,
        reversed_by: None,
        reason: None,
    }
}
fn reverse_receipt(doc: &mut Document, actor: i64, reason: &str) {
    for p in doc.payments.iter_mut().filter(|p| p.reversed_at.is_none()) {
        p.reversed_at = Some(chrono::Utc::now().to_rfc3339());
        p.reversed_by = Some(actor.to_string());
        p.reason = Some(reason.into());
    }
}

pub(crate) async fn command_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    d: &str,
    cmd: Command,
) -> Result<Document> {
    command_inner(tx, a, c, d, cmd, true).await
}
pub(crate) async fn command_without_cash_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    d: &str,
    cmd: Command,
) -> Result<Document> {
    command_inner(tx, a, c, d, cmd, false).await
}
async fn command_inner(
    tx: &mut Transaction<'_, Postgres>,
    a: &str,
    c: &str,
    d: &str,
    cmd: Command,
    automatic: bool,
) -> Result<Document> {
    let (a, c) = lock(tx, a, c).await?;
    let input = serde_json::json!({"id":d,"command":&cmd});
    if let Some(v) = replay(tx, a, c, &cmd.request_id, &input).await? {
        return Ok(v);
    }
    let mut v = load(tx, c, id(d)?).await?;
    if v.revision != cmd.revision || v.cancelled {
        return Err(Error::Conflict);
    }
    let bank_payment = match &cmd.action {
        Action::Reverse { payment_id, .. } => Some(payment_id.as_str()),
        _ => None,
    };
    if !matches!(cmd.action,Action::Pay{..}) && sqlx::query_scalar::<_,bool>(
            "SELECT EXISTS(SELECT 1 FROM bank_match WHERE company_id=$1 AND received_id=$2 AND detached_at IS NULL AND ($3::text IS NULL OR payment_id=$3))"
        ).bind(c).bind(id(d)?).bind(bank_payment).fetch_one(&mut **tx).await.map_err(err)? {
            return Err(Error::Invalid(vec!["bank_link".into()]));
        }
    let cash_reason = match &cmd.action {
        Action::Reverse { reason, .. } | Action::Cancel { reason } => Some(reason.as_str()),
        Action::Update { .. } if v.data.kind == Kind::Receipt => Some("receipt_update"),
        _ => None,
    };
    if let Some(reason) = cash_reason {
        crate::cash_automation::reverse(tx, a, c, "received", id(d)?, bank_payment, reason).await?;
    }
    if !matches!(cmd.action,Action::Pay{..}) && sqlx::query_scalar::<_,bool>(
            "SELECT EXISTS(SELECT 1 FROM cash_match WHERE company_id=$1 AND received_id=$2 AND detached_at IS NULL AND ($3::text IS NULL OR payment_id=$3))"
        ).bind(c).bind(id(d)?).bind(bank_payment).fetch_one(&mut **tx).await.map_err(err)? {
            return Err(Error::Invalid(vec!["cash_link".into()]));
        }
    let paid = amount(&v.paid).ok_or(Error::Unavailable)?;
    let action = match &cmd.action {
        Action::Update { data } => {
            if data.kind != v.data.kind || (paid > Decimal::ZERO && v.data.kind != Kind::Receipt) {
                return Err(Error::Conflict);
            }
            contact(tx, c, data).await?;
            v.totals = totals(data).map_err(Error::Invalid)?;
            v.data = data.as_ref().clone();
            if v.data.kind == Kind::Receipt {
                reverse_receipt(&mut v, a, "receipt_update");
                v.payments.push(receipt_payment(
                    &cmd.request_id,
                    a,
                    &v.data,
                    &v.totals.total,
                ));
            }
            "update"
        }
        Action::Pay { payment } => {
            if v.data.currency != "CZK" && payment.method == "cash" {
                return Err(Error::Invalid(vec!["currency".into()]));
            }
            if v.data.kind == Kind::Receipt {
                return Err(Error::Conflict);
            }
            let value =
                amount(&payment.amount).ok_or_else(|| Error::Invalid(vec!["amount".into()]))?;
            if value > amount(&v.remaining).ok_or(Error::Unavailable)? {
                return Err(Error::Invalid(vec!["amount".into()]));
            }
            v.payments.push(Payment {
                id: cmd.request_id.clone(),
                input: payment.clone(),
                actor_id: a.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                reversed_at: None,
                reversed_by: None,
                reason: None,
            });
            "pay"
        }
        Action::Reverse { payment_id, reason } => {
            if v.data.kind == Kind::Receipt {
                return Err(Error::Conflict);
            }
            let p = v
                .payments
                .iter_mut()
                .find(|p| p.id == *payment_id)
                .ok_or(Error::NotFound)?;
            if p.reversed_at.is_some() {
                return Err(Error::Conflict);
            }
            p.reversed_at = Some(chrono::Utc::now().to_rfc3339());
            p.reversed_by = Some(a.to_string());
            p.reason = Some(reason.clone());
            "reverse"
        }
        Action::Cancel { reason } => {
            if paid > Decimal::ZERO && v.data.kind != Kind::Receipt {
                return Err(Error::Conflict);
            }
            if v.data.kind == Kind::Receipt {
                reverse_receipt(&mut v, a, reason);
            }
            v.cancelled = true;
            "cancel"
        }
    };
    let paid = v
        .payments
        .iter()
        .filter(|p| p.reversed_at.is_none())
        .try_fold(Decimal::ZERO, |s, p| amount(&p.input.amount).map(|a| s + a))
        .ok_or(Error::Unavailable)?;
    let data = &v.data;
    let r=sqlx::query("UPDATE received_invoice SET revision=revision+1,cancelled=$3,data=$4,payments=$5,total=$6,paid=$7,supplier_contact_id=$8,received_on=$9,due_on=$10 WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(d)?).bind(v.cancelled).bind(serde_json::to_value(data).map_err(err)?).bind(serde_json::to_value(&v.payments).map_err(err)?).bind(amount(&v.totals.total).ok_or(Error::Unavailable)?).bind(paid).bind(data.supplier_contact_id.as_deref().map(id).transpose()?).bind(data.received_on).bind(data.due_on).fetch_one(&mut **tx).await.map_err(err)?;
    let doc = map(r)?;
    if automatic {
        for p in doc
            .payments
            .iter()
            .filter(|p| p.id == cmd.request_id && p.reversed_at.is_none())
        {
            crate::cash_automation::record(
                tx,
                a,
                c,
                crate::cash_automation::Source {
                    kind: "received",
                    document: id(d)?,
                    payment_id: &p.id,
                    payment: &p.input,
                },
            )
            .await?;
        }
    }
    if matches!(cmd.action, Action::Update { .. } | Action::Cancel { .. }) {
        crate::stock_documents::received(tx, a, c, id(d)?, &doc.data, doc.cancelled).await?;
    }
    record(tx, a, c, &cmd.request_id, input, &doc, action).await?;
    Ok(doc)
}

pub(crate) async fn create_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    cmd: &Create,
    input: serde_json::Value,
) -> Result<Document> {
    contact(tx, c, &cmd.data).await?;
    let t = totals(&cmd.data).map_err(Error::Invalid)?;
    let year = cmd.data.received_on.year();
    if !(1900..=9999).contains(&year) {
        return Err(Error::Invalid(vec!["received_on".into()]));
    }
    let seq:i64=sqlx::query_scalar("INSERT INTO received_series(company_id,year,next_value,kind) VALUES($1,$2,2,$3) ON CONFLICT(company_id,year,kind) DO UPDATE SET next_value=received_series.next_value+1 RETURNING next_value-1").bind(c).bind(year).bind(cmd.data.kind.as_str()).fetch_one(&mut **tx).await.map_err(err)?;
    let prefix = if cmd.data.kind == Kind::Receipt {
        "UC"
    } else {
        "PF"
    };
    let number = format!("{prefix}{year}{seq:06}");
    let d = &cmd.data;
    let row=sqlx::query("INSERT INTO received_invoice(company_id,supplier_contact_id,number,data,total,received_on,due_on,kind) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *").bind(c).bind(d.supplier_contact_id.as_deref().map(id).transpose()?).bind(number).bind(serde_json::to_value(d).map_err(err)?).bind(amount(&t.total).ok_or(Error::Unavailable)?).bind(d.received_on).bind(d.due_on).bind(d.kind.as_str()).fetch_one(&mut **tx).await.map_err(err)?;
    let mut doc = map(row)?;
    if doc.data.kind == Kind::Receipt {
        doc.payments.push(receipt_payment(
            &cmd.request_id,
            a,
            &doc.data,
            &doc.totals.total,
        ));
        let row = sqlx::query("UPDATE received_invoice SET paid=total,payments=$3 WHERE company_id=$1 AND id=$2 RETURNING *")
                .bind(c).bind(id(&doc.id)?).bind(serde_json::to_value(&doc.payments).map_err(err)?)
                .fetch_one(&mut **tx).await.map_err(err)?;
        doc = map(row)?;
    }
    for payment in &doc.payments {
        crate::cash_automation::record(
            tx,
            a,
            c,
            crate::cash_automation::Source {
                kind: "received",
                document: id(&doc.id)?,
                payment_id: &payment.id,
                payment: &payment.input,
            },
        )
        .await?;
    }
    crate::stock_documents::received(tx, a, c, id(&doc.id)?, &doc.data, false).await?;
    record(tx, a, c, &cmd.request_id, input, &doc, "create").await?;

    Ok(doc)
}

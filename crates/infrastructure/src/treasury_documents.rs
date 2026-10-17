//! Shared advance receipts and credit refunds for bank and cash matching.
use faktura_application::{
    advance,
    banking::Candidate,
    company::{CompanyError as Error, Result},
    credit_note,
};
use faktura_domain::received::{amount, money};
use rust_decimal::Decimal;
use sqlx::{Postgres, Row, Transaction};
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
pub(crate) async fn linked(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    kind: &str,
    d: i64,
    p: &str,
) -> Result<bool> {
    let column = match kind {
        "advance" => "advance_id",
        "credit" => "credit_id",
        _ => return Err(Error::NotFound),
    };
    sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM bank_match WHERE company_id=$1 AND {column}=$2 AND payment_id=$3 AND detached_at IS NULL UNION ALL SELECT 1 FROM cash_match WHERE company_id=$1 AND {column}=$2 AND payment_id=$3 AND detached_at IS NULL)")).bind(c).bind(d).bind(p).fetch_one(&mut **tx).await.map_err(err)
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn candidates(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    incoming: bool,
    available: Decimal,
    on: chrono::NaiveDate,
    method: &str,
    q: &str,
    only: Option<i64>,
    partial: bool,
) -> Result<Vec<Candidate>> {
    let mut result = vec![];
    if incoming {
        let rows=sqlx::query("SELECT id,number,revision,data,(advance_totals->>'payable') AS advance_total,advance_paid::text FROM sales_document WHERE company_id=$1 AND kind='advance' AND NOT archived AND ($3::bigint IS NULL OR id=$3) AND ($2='' OR position(lower($2) in lower(number||' '||coalesce(data->'invoice'->'customer'->>'name','')))>0) ORDER BY id DESC LIMIT 50").bind(c).bind(q).bind(only).fetch_all(&mut **tx).await.map_err(err)?;
        for r in rows {
            let d: i64 = r.get("id");
            match crate::advance_settlement::ensure_free(tx, c, d).await {
                Ok(()) => {}
                Err(Error::Conflict) => continue,
                Err(e) => return Err(e),
            }
            let data: faktura_application::sales::Data =
                serde_json::from_value(r.get("data")).map_err(err)?;
            if data.invoice.calculation.currency != "CZK" {
                continue;
            }
            let outstanding = (amount(&r.get::<String, _>("advance_total"))
                .ok_or(Error::Unavailable)?
                - amount(&r.get::<String, _>("advance_paid")).ok_or(Error::Unavailable)?)
            .max(Decimal::ZERO);
            let base = Candidate {
                target_kind: "advance".into(),
                document_kind: "advance".into(),
                document_id: d.to_string(),
                number: r.get("number"),
                partner: data.invoice.customer.map(|p| p.name).unwrap_or_default(),
                revision: r.get("revision"),
                payment_id: None,
                outstanding: money(outstanding),
                available: money(outstanding.min(available)),
            };
            if data.invoice.issued_on.is_some_and(|v| v <= on)
                && outstanding > Decimal::ZERO
                && (partial || outstanding >= available)
            {
                result.push(base.clone());
            }
            let payments=sqlx::query("SELECT id,amount,paid_on FROM advance_payment WHERE company_id=$1 AND document_id=$2 AND method=$3 AND reversed_at IS NULL").bind(c).bind(d).bind(method).fetch_all(&mut **tx).await.map_err(err)?;
            for p in payments {
                if method == "cash" && p.get::<chrono::NaiveDate, _>("paid_on") != on {
                    continue;
                }
                let value: Decimal = p.get("amount");
                let payment = p.get::<i64, _>("id").to_string();
                if (if partial {
                    value <= available
                } else {
                    value == available
                }) && !linked(tx, c, "advance", d, &payment).await?
                {
                    let mut v = base.clone();
                    v.payment_id = Some(payment);
                    v.available = money(value);
                    result.push(v);
                }
            }
        }
    } else {
        let ids:Vec<i64>=sqlx::query_scalar("SELECT id FROM credit_note WHERE company_id=$1 AND status='issued' AND ($3::bigint IS NULL OR id=$3) AND ($2='' OR position(lower($2) in lower(number||' '||coalesce(source->'data'->'customer'->>'name','')))>0) ORDER BY id DESC LIMIT 50").bind(c).bind(q).bind(only).fetch_all(&mut **tx).await.map_err(err)?;
        for d in ids {
            let doc = crate::credit_note::load(tx, c, d).await?;
            let outstanding = amount(&doc.remaining).ok_or(Error::Unavailable)?;
            let base = Candidate {
                target_kind: "credit".into(),
                document_kind: "credit".into(),
                document_id: d.to_string(),
                number: doc.number.ok_or(Error::Conflict)?,
                partner: doc.source.data.customer.map(|p| p.name).unwrap_or_default(),
                revision: doc.revision,
                payment_id: None,
                outstanding: doc.remaining,
                available: money(outstanding.min(available)),
            };
            if doc.data.issued_on <= on
                && outstanding > Decimal::ZERO
                && (partial || outstanding >= available)
            {
                result.push(base.clone());
            }
            for event in &doc.events {
                if (method == "cash" && event.date != on.to_string())
                    || event.kind != "refund"
                    || event.method != method
                    || doc
                        .events
                        .iter()
                        .any(|e| e.reverses.as_ref() == Some(&event.id))
                {
                    continue;
                }
                let value = amount(&event.amount).ok_or(Error::Unavailable)?;
                if (if partial {
                    value <= available
                } else {
                    value == available
                }) && !linked(tx, c, "credit", d, &event.id).await?
                {
                    let mut v = base.clone();
                    v.payment_id = Some(event.id.clone());
                    v.available = money(value);
                    result.push(v);
                }
            }
        }
    }
    Ok(result)
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn pay(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    kind: &str,
    doc: &str,
    revision: i64,
    value: Decimal,
    on: chrono::NaiveDate,
    method: &str,
    note: &str,
) -> Result<String> {
    let request_id = uuid::Uuid::new_v4().to_string();
    if kind == "advance" {
        crate::advance::register_in_tx(
            tx,
            actor,
            company,
            doc,
            advance::Register {
                request_id: request_id.clone(),
                revision,
                payment: faktura_domain::payment::PaymentInput {
                    amount: money(value),
                    paid_on: on,
                    method: method.into(),
                    note: note.into(),
                },
            },
        )
        .await?;
        sqlx::query_scalar::<_,i64>("SELECT id FROM advance_payment WHERE company_id=$1::text::bigint AND request_id=$2::text::uuid").bind(company).bind(request_id).fetch_one(&mut **tx).await.map(|v|v.to_string()).map_err(err)
    } else {
        let before = crate::credit_note::load(tx, id(company)?, id(doc)?).await?;
        let after = crate::credit_note::command_in_tx(
            tx,
            actor,
            company,
            doc,
            credit_note::Command {
                request_id,
                revision,
                action: credit_note::Action::Refund {
                    amount: money(value),
                    date: on,
                    method: method.into(),
                    reason: note.into(),
                },
            },
        )
        .await?;
        after
            .events
            .iter()
            .find(|e| !before.events.iter().any(|b| b.id == e.id))
            .map(|v| v.id.clone())
            .ok_or(Error::Unavailable)
    }
}
pub(crate) async fn reverse(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    kind: &str,
    doc: &str,
    payment: &str,
    reason: &str,
) -> Result<()> {
    if kind == "advance" {
        let revision=sqlx::query_scalar("SELECT revision FROM sales_document WHERE company_id=$1::text::bigint AND id=$2::text::bigint").bind(company).bind(doc).fetch_one(&mut **tx).await.map_err(err)?;
        crate::advance::reverse_in_tx(
            tx,
            actor,
            company,
            doc,
            payment,
            advance::Reverse {
                revision,
                reason: reason.into(),
            },
        )
        .await
    } else {
        let d = crate::credit_note::load(tx, id(company)?, id(doc)?).await?;
        crate::credit_note::command_in_tx(
            tx,
            actor,
            company,
            doc,
            credit_note::Command {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: d.revision,
                action: credit_note::Action::Reverse {
                    event_id: payment.into(),
                    reason: reason.into(),
                },
            },
        )
        .await
        .map(|_| ())
    }
}

// Explicit allocation between different currencies. Never guessed by automation.
use super::*;
use faktura_domain::exchange::{to_czk, ExchangeRate};
pub(super) async fn candidates(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    m: &Movement,
    q: &str,
    only: Option<i64>,
) -> Result<Vec<ForeignCandidate>> {
    if q.len() > 200 {
        return Err(invalid("query"));
    }
    let account = get_account(tx, c, &m.account_id).await?;
    if account.archived || m.remaining == "0.00" {
        return Ok(vec![]);
    }
    let incoming = !m.data.amount.starts_with('-');
    let (table, filter, partner, currency) = if incoming {
        (
            "invoice_draft",
            "number IS NOT NULL",
            "customer",
            "data->'calculation'->>'currency'",
        )
    } else {
        (
            "received_invoice",
            "NOT cancelled AND kind='invoice'",
            "supplier",
            "data->>'currency'",
        )
    };
    let rows=sqlx::query(&format!("SELECT id,number,data FROM {table} WHERE company_id=$1 AND ($3::bigint IS NULL OR id=$3) AND {filter} AND {currency}<>$4 AND ($2='' OR position(lower($2) in lower(number||' '||coalesce(data->'{partner}'->>'name','')))>0) ORDER BY id DESC LIMIT 50"))
        .bind(c).bind(q).bind(only).bind(&m.data.currency).fetch_all(&mut **tx).await.map_err(err)?;
    let mut result = vec![];
    for row in rows {
        let doc: i64 = row.get("id");
        let data: serde_json::Value = row.get("data");
        if !data["issued_on"]
            .as_str()
            .is_some_and(|d| d <= m.data.booked_on.to_string().as_str())
        {
            continue;
        }
        let (remaining, revision, currency, existing) = if incoming {
            let payments = crate::payment::load(tx, c, doc).await?;
            (
                payments.summary.outstanding,
                payments.summary.revision,
                data["calculation"]["currency"].as_str().unwrap_or_default(),
                payments
                    .items
                    .into_iter()
                    .filter(|p| p.reversed_at.is_none() && p.method == "bank_transfer")
                    .map(|p| (p.id, p.amount))
                    .collect::<Vec<_>>(),
            )
        } else {
            let d = crate::received::load(tx, c, doc).await?;
            (
                d.remaining,
                d.revision,
                data["currency"].as_str().unwrap_or_default(),
                d.payments
                    .into_iter()
                    .filter(|p| p.reversed_at.is_none() && p.input.method == "bank_transfer")
                    .map(|p| (p.id, p.input.amount))
                    .collect::<Vec<_>>(),
            )
        };
        if amount(&remaining).is_some_and(|v| v > rust_decimal::Decimal::ZERO) {
            result.push(ForeignCandidate {
                payment_id: None,
                payment_amount: None,
                target_kind: if incoming { "invoice" } else { "received" }.into(),
                document_id: doc.to_string(),
                number: row.get("number"),
                partner: data[partner]["name"].as_str().unwrap_or_default().into(),
                revision,
                outstanding: remaining.clone(),
                currency: currency.into(),
            });
        }
        for (payment_id, payment_amount) in existing {
            let kind = if incoming { "invoice" } else { "received" };
            if !linked(tx, c, kind, doc, &payment_id).await? {
                result.push(ForeignCandidate {
                    payment_id: Some(payment_id),
                    payment_amount: Some(payment_amount),
                    target_kind: kind.into(),
                    document_id: doc.to_string(),
                    number: row.get("number"),
                    partner: data[partner]["name"].as_str().unwrap_or_default().into(),
                    revision,
                    outstanding: remaining.clone(),
                    currency: currency.into(),
                });
            }
        }
    }
    let (table, currency_path, filter, partner) = if incoming {
        (
            "sales_document",
            "data->'invoice'->'calculation'->>'currency'",
            "kind='advance' AND NOT archived",
            "data->'invoice'->'customer'->>'name'",
        )
    } else {
        (
            "credit_note",
            "totals->>'currency'",
            "status='issued'",
            "source->'data'->'customer'->>'name'",
        )
    };
    let rows=sqlx::query(&format!("SELECT id,{currency_path} AS currency FROM {table} WHERE company_id=$1 AND {filter} AND {currency_path}<>$4 AND ($3::bigint IS NULL OR id=$3) AND ($2='' OR position(lower($2) in lower(number||' '||coalesce({partner},'')))>0) ORDER BY id DESC LIMIT 50"))
        .bind(c).bind(q).bind(only).bind(&m.data.currency).fetch_all(&mut **tx).await.map_err(err)?;
    for row in rows {
        let currency: String = row.get("currency");
        let choices = crate::treasury_documents::candidates(
            tx,
            c,
            incoming,
            rust_decimal::Decimal::MAX,
            m.data.booked_on,
            "bank_transfer",
            &currency,
            "",
            Some(row.get("id")),
            true,
        )
        .await?;
        for choice in choices {
            result.push(ForeignCandidate {
                payment_amount: choice.payment_id.as_ref().map(|_| choice.available.clone()),
                payment_id: choice.payment_id,
                target_kind: choice.target_kind,
                document_id: choice.document_id,
                number: choice.number,
                partner: choice.partner,
                revision: choice.revision,
                outstanding: choice.outstanding,
                currency: currency.clone(),
            });
        }
    }
    Ok(result)
}
pub(super) async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    a: i64,
    c: i64,
    action: Action,
) -> Result<Movement> {
    let Action::ForeignMatch {
        bank_rate,
        payment_id,
        movement_id,
        revision,
        target_kind,
        document_id,
        target_revision,
        bank_amount,
        document_amount,
    } = action
    else {
        return Err(invalid("action"));
    };
    let m = movement(tx, c, id(&movement_id)?).await?;
    if m.revision != revision {
        return Err(Error::Conflict);
    }
    let allocated = amount(&bank_amount)
        .filter(|v| *v > rust_decimal::Decimal::ZERO)
        .ok_or_else(|| invalid("bank_amount"))?;
    let paid = amount(&document_amount)
        .filter(|v| *v > rust_decimal::Decimal::ZERO)
        .ok_or_else(|| invalid("document_amount"))?;
    if allocated > amount(&m.remaining).ok_or(Error::Unavailable)? {
        return Err(invalid("bank_amount"));
    }
    let incoming = !m.data.amount.starts_with('-');
    if !(if incoming {
        ["invoice", "advance"]
    } else {
        ["received", "credit"]
    })
    .contains(&target_kind.as_str())
    {
        return Err(invalid("target_kind"));
    }
    let (table, data_path) = match target_kind.as_str() {
        "invoice" => ("invoice_draft", "data"),
        "received" => ("received_invoice", "data"),
        "advance" => ("sales_document", "data->'invoice'"),
        "credit" => ("credit_note", "source->'data'"),
        _ => return Err(invalid("target_kind")),
    };
    let json: serde_json::Value = sqlx::query_scalar(&format!(
        "SELECT {data_path} FROM {table} WHERE company_id=$1 AND id=$2 FOR UPDATE"
    ))
    .bind(c)
    .bind(id(&document_id)?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    .ok_or(Error::NotFound)?;
    let candidate = candidates(tx, c, &m, "", Some(id(&document_id)?))
        .await?
        .into_iter()
        .find(|candidate| {
            candidate.target_kind == target_kind
                && candidate.document_id == document_id
                && candidate.payment_id == payment_id
        })
        .ok_or(Error::Conflict)?;
    if candidate.revision != target_revision
        || if let Some(value) = &candidate.payment_amount {
            Some(paid) != amount(value)
        } else {
            paid > amount(&candidate.outstanding).ok_or(Error::Unavailable)?
        }
    {
        return Err(Error::Conflict);
    }
    let rate: Option<ExchangeRate> = if candidate.currency == "CZK" {
        None
    } else {
        Some(
            serde_json::from_value(if target_kind != "received" {
                json["calculation"]["exchange_rate"].clone()
            } else {
                json["exchange_rate"].clone()
            })
            .map_err(err)?,
        )
    };
    let book = to_czk(paid, &candidate.currency, rate.as_ref()).ok_or(Error::Unavailable)?;
    let bank_value = to_czk(allocated, &m.data.currency, bank_rate.as_ref())
        .ok_or_else(|| invalid("bank_rate"))?;
    let snapshot = ForeignSettlement {
        bank_currency: m.data.currency.clone(),
        bank_rate,
        bank_value_czk: Some(money(bank_value)),
        currency: candidate.currency,
        document_amount: money(paid),
        document_rate: rate.unwrap_or(ExchangeRate {
            rate: "1".into(),
            units: "1".into(),
        }),
        book_value_czk: money(book),
        difference_czk: money(if incoming {
            bank_value - book
        } else {
            book - bank_value
        }),
    };
    let payment = PaymentInput {
        amount: money(paid),
        paid_on: m.data.booked_on,
        method: "bank_transfer".into(),
        note: m.number.clone(),
    };
    let created = payment_id.is_none();
    let payment_id = if let Some(payment_id) = payment_id {
        payment_id
    } else if matches!(target_kind.as_str(), "advance" | "credit") {
        crate::treasury_documents::pay(
            tx,
            actor,
            company,
            &target_kind,
            &document_id,
            target_revision,
            paid,
            m.data.booked_on,
            "bank_transfer",
            &m.number,
        )
        .await?
    } else if incoming {
        let before = crate::payment::load(tx, c, id(&document_id)?).await?;
        let after = crate::payment::register_in_tx(
            tx,
            actor,
            company,
            &document_id,
            payment::Register {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: target_revision,
                payment,
            },
        )
        .await?;
        after
            .items
            .iter()
            .find(|p| !before.items.iter().any(|b| b.id == p.id))
            .ok_or(Error::Unavailable)?
            .id
            .clone()
    } else {
        let key = uuid::Uuid::new_v4().to_string();
        crate::received::command_in_tx(
            tx,
            actor,
            company,
            &document_id,
            received::Command {
                request_id: key.clone(),
                revision: target_revision,
                action: received::Action::Pay { payment },
            },
        )
        .await?;
        key
    };
    sqlx::query("INSERT INTO bank_match(company_id,movement_id,invoice_id,received_id,payment_id,created_payment,actor_id,amount,foreign_settlement,advance_id,credit_id) VALUES($1,$2,$3,$4,$5,$9,$6,$7,$8,$10,$11)")
        .bind(c).bind(id(&m.id)?).bind(if target_kind == "invoice" {Some(id(&document_id)?)} else {None}).bind(if target_kind == "received" {Some(id(&document_id)?)} else {None}).bind(payment_id).bind(a).bind(allocated).bind(serde_json::to_value(snapshot).map_err(err)?).bind(created).bind(if target_kind == "advance" {Some(id(&document_id)?)} else {None}).bind(if target_kind == "credit" {Some(id(&document_id)?)} else {None}).execute(&mut **tx).await.map_err(err)?;
    sqlx::query("UPDATE bank_movement SET revision=revision+1 WHERE company_id=$1 AND id=$2")
        .bind(c)
        .bind(id(&m.id)?)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    movement(tx, c, id(&m.id)?).await
}

pub(super) async fn report(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    q: ForeignReportQuery,
) -> Result<ForeignReport> {
    let source="FROM bank_match b JOIN bank_movement m ON m.company_id=b.company_id AND m.id=b.movement_id WHERE b.company_id=$1 AND b.detached_at IS NULL AND b.foreign_settlement IS NOT NULL AND m.booked_on BETWEEN $2 AND $3";
    let row=sqlx::query(&format!("SELECT count(*) total,coalesce(sum(greatest((b.foreign_settlement->>'difference_czk')::numeric,0)),0) gains,coalesce(sum(greatest(-(b.foreign_settlement->>'difference_czk')::numeric,0)),0) losses {source}")).bind(c).bind(q.from).bind(q.to).fetch_one(&mut **tx).await.map_err(err)?;
    let gains: rust_decimal::Decimal = row.get("gains");
    let losses: rust_decimal::Decimal = row.get("losses");
    let rows=sqlx::query(&format!("SELECT b.id,b.movement_id,m.number movement_number,m.booked_on,b.invoice_id,b.received_id,b.advance_id,b.credit_id,b.amount,b.foreign_settlement,coalesce((SELECT number FROM invoice_draft WHERE company_id=b.company_id AND id=b.invoice_id),(SELECT number FROM received_invoice WHERE company_id=b.company_id AND id=b.received_id),(SELECT number FROM sales_document WHERE company_id=b.company_id AND id=b.advance_id),(SELECT number FROM credit_note WHERE company_id=b.company_id AND id=b.credit_id)) document_number {source} ORDER BY m.booked_on DESC,b.id DESC LIMIT 25 OFFSET $4")).bind(c).bind(q.from).bind(q.to).bind((q.page.unwrap_or(1)-1)*25).fetch_all(&mut **tx).await.map_err(err)?;
    let items = rows
        .into_iter()
        .map(|r| {
            let invoice: Option<i64> = r.get("invoice_id");
            let received: Option<i64> = r.get("received_id");
            let advance: Option<i64> = r.get("advance_id");
            let credit: Option<i64> = r.get("credit_id");
            Ok(ForeignReportEntry {
                id: r.get::<i64, _>("id").to_string(),
                movement_id: r.get::<i64, _>("movement_id").to_string(),
                movement_number: r.get("movement_number"),
                document_id: invoice
                    .or(received)
                    .or(advance)
                    .or(credit)
                    .ok_or(Error::Unavailable)?
                    .to_string(),
                document_number: r.get("document_number"),
                target_kind: if invoice.is_some() {
                    "invoice"
                } else if advance.is_some() {
                    "advance"
                } else if credit.is_some() {
                    "credit"
                } else {
                    "received"
                }
                .into(),
                booked_on: r.get("booked_on"),
                bank_amount: money(r.get("amount")),
                settlement: serde_json::from_value(r.get("foreign_settlement")).map_err(err)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ForeignReport {
        gains: money(gains),
        losses: money(losses),
        net: money(gains - losses),
        currency: "CZK".into(),
        total: row.get("total"),
        page: q.page.unwrap_or(1),
        items,
    })
}

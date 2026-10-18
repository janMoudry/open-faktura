//! Document stock effects. One transaction owns the source and all card changes.
use faktura_application::company::{CompanyError as Error, Result};
use rust_decimal::Decimal;
use sqlx::{Postgres, Row, Transaction};
use std::{collections::BTreeMap, str::FromStr};
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn invalid() -> Error {
    Error::Invalid(vec!["stock".into()])
}
pub(crate) fn quantity(s: &str) -> Result<Decimal> {
    if s.is_empty() || s.len() > 20 || !s.bytes().all(|v| v.is_ascii_digit() || v == b'.') {
        return Err(invalid());
    }
    let q = Decimal::from_str(s).map_err(|_| invalid())?;
    if q <= Decimal::ZERO || q.scale() > 4 || q > Decimal::from(1_000_000) {
        return Err(invalid());
    }
    Ok(q)
}
pub(crate) fn acquisition_cost(s: Option<&str>) -> Result<Option<Decimal>> {
    s.map(|s| {
        if s.is_empty() || s.len() > 20 || !s.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
            return Err(invalid());
        }
        let v = Decimal::from_str(s).map_err(|_| invalid())?;
        if v < Decimal::ZERO || v > Decimal::from(1_000_000_000) || v.scale() > 4 {
            return Err(invalid());
        }
        Ok(v)
    })
    .transpose()
}
pub(crate) struct Entry {
    pub line: String,
    pub catalog: String,
    pub quantity: Decimal,
    pub unit: String,
    pub acquisition_unit_cost: Option<Decimal>,
    pub extra_cost: Decimal,
}
/// Quantities are desired signed totals, not deltas. Revisions append differences.
pub(crate) async fn sync(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    company: i64,
    kind: &str,
    document: i64,
    date: chrono::NaiveDate,
    entries: Vec<Entry>,
) -> Result<()> {
    let column = match kind {
        "retail" => "retail_id",
        "invoice" => "invoice_id",
        "received" => "received_id",
        "credit" => "credit_id",
        _ => return Err(invalid()),
    };
    if entries.is_empty() {
        // Upgrade tests seed older schemas with current commands. No linked
        // quantities can exist before the source column was introduced.
        let available: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid=to_regclass('stock_movement') AND attname=$1 AND NOT attisdropped)").bind(column).fetch_one(&mut **tx).await.map_err(err)?;
        if !available {
            return Ok(());
        }
    }
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    let mut desired = BTreeMap::new();
    for e in entries {
        let catalog: i64 = e.catalog.parse().map_err(|_| invalid())?;
        if desired
            .insert(
                (catalog, e.line),
                (e.quantity, e.unit, e.acquisition_unit_cost, e.extra_cost),
            )
            .is_some()
        {
            return Err(invalid());
        }
    }
    let old=sqlx::query(&format!("SELECT catalog_id,source_line,sum(quantity) AS quantity FROM stock_movement WHERE company_id=$1 AND {column}=$2 GROUP BY catalog_id,source_line"))
        .bind(company).bind(document).fetch_all(&mut **tx).await.map_err(err)?;
    let previous: BTreeMap<(i64, String), Decimal> = old
        .into_iter()
        .map(|r| {
            (
                (r.get("catalog_id"), r.get("source_line")),
                r.get("quantity"),
            )
        })
        .collect();
    for k in previous.keys() {
        desired
            .entry(k.clone())
            .or_insert((Decimal::ZERO, String::new(), None, Decimal::ZERO));
    }
    for ((catalog, line), (wanted, unit, acquisition, extra)) in desired {
        let before = previous
            .get(&(catalog, line.clone()))
            .copied()
            .unwrap_or_default();
        let delta = wanted - before;
        let previous_cost = if kind == "received" && before != Decimal::ZERO {
            sqlx::query("SELECT acquisition_unit_cost,acquisition_extra_cost FROM stock_movement WHERE company_id=$1 AND catalog_id=$2 AND received_id=$3 AND source_line=$4 ORDER BY id DESC LIMIT 1")
                .bind(company).bind(catalog).bind(document).bind(&line)
                .fetch_optional(&mut **tx).await.map_err(err)?
                .map(|r| (r.get::<Option<Decimal>, _>("acquisition_unit_cost"),r.get::<Decimal,_>("acquisition_extra_cost")))
        } else {
            None
        };
        let cost_changed = kind == "received"
            && before != Decimal::ZERO
            && previous_cost != Some((acquisition, extra));
        if delta == Decimal::ZERO && !cost_changed {
            continue;
        }
        let row=sqlx::query("SELECT s.quantity,s.opened_on,c.data,c.archived FROM catalog c LEFT JOIN stock_card s ON s.company_id=c.company_id AND s.catalog_id=c.id WHERE c.company_id=$1 AND c.id=$2 FOR SHARE OF c")
            .bind(company).bind(catalog).fetch_optional(&mut **tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        // Selecting an ordinary catalog item does not opt it into stock tracking.
        if row.get::<Option<Decimal>, _>("quantity").is_none() {
            if matches!(kind, "invoice" | "retail") {
                continue;
            }
            return Err(invalid());
        }
        let item: serde_json::Value = row.get("data");
        if wanted != Decimal::ZERO
            && (row.get::<bool, _>("archived") || item["unit"].as_str() != Some(unit.as_str()))
        {
            return Err(invalid());
        }
        let card=sqlx::query("SELECT quantity,opened_on FROM stock_card WHERE company_id=$1 AND catalog_id=$2 FOR UPDATE").bind(company).bind(catalog).fetch_one(&mut **tx).await.map_err(err)?;
        if date > today
            || date < card.get::<chrono::NaiveDate, _>("opened_on")
            || delta.abs() > Decimal::from(1_000_000)
        {
            return Err(invalid());
        }
        let balance = card.get::<Decimal, _>("quantity") + delta;
        if balance.abs() > Decimal::from(1_000_000_000) {
            return Err(invalid());
        }
        sqlx::query("UPDATE stock_card SET quantity=$3,revision=revision+1 WHERE company_id=$1 AND catalog_id=$2").bind(company).bind(catalog).bind(balance).execute(&mut **tx).await.map_err(err)?;
        let correction = kind == "received" && before != Decimal::ZERO;
        sqlx::query(&format!("INSERT INTO stock_movement(company_id,catalog_id,quantity,booked_on,kind,reason,item,actor_id,{column},source_line,acquisition_unit_cost,acquisition_correction,acquisition_quantity,acquisition_extra_cost) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)"))
            .bind(company).bind(catalog).bind(delta).bind(date).bind(kind).bind(format!("{kind} #{document} / {line}")).bind(item).bind(actor).bind(document).bind(line).bind(acquisition).bind(correction).bind(if kind=="received" {Some(wanted)} else {None}).bind(extra).execute(&mut **tx).await.map_err(err)?;
    }
    Ok(())
}
pub(crate) async fn invoice(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    id: i64,
    d: &faktura_domain::invoice::DraftData,
) -> Result<()> {
    let entries = d
        .calculation
        .lines
        .iter()
        .filter(|l| l.catalog_id.is_some())
        .map(|l| {
            Ok(Entry {
                line: l.id.clone(),
                catalog: l.catalog_id.clone().unwrap(),
                quantity: -quantity(&l.quantity)?,
                unit: l.unit.clone(),
                acquisition_unit_cost: None,
                extra_cost: Decimal::ZERO,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    sync(
        tx,
        a,
        c,
        "invoice",
        id,
        d.issued_on.ok_or_else(invalid)?,
        entries,
    )
    .await
}
pub(crate) async fn received(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    id: i64,
    d: &faktura_domain::received::Data,
    cancelled: bool,
) -> Result<()> {
    let entries = if cancelled {
        vec![]
    } else {
        d.lines
            .iter()
            .enumerate()
            .filter_map(|(n, l)| l.stock.as_ref().map(|s| (n, s)))
            .map(|(n, s)| {
                Ok(Entry {
                    line: n.to_string(),
                    catalog: s.catalog_id.clone(),
                    quantity: quantity(&s.quantity)?,
                    unit: s.unit.clone(),
                    acquisition_unit_cost: acquisition_cost(s.acquisition_unit_cost.as_deref())?,
                    extra_cost: Decimal::ZERO,
                })
            })
            .collect::<Result<Vec<_>>>()?
    };
    let mut entries = entries;
    if !cancelled {
        let total = acquisition_cost(d.stock_incidental_cost.as_deref())?.unwrap_or_default();
        if total.scale() > 2 {
            return Err(invalid());
        }
        let weights = entries
            .iter()
            .map(|e| {
                if total.is_zero() {
                    Ok(Decimal::ZERO)
                } else {
                    e.acquisition_unit_cost
                        .map(|p| p * e.quantity)
                        .ok_or_else(invalid)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let allocation =
            faktura_domain::stock_cost::allocate(total, &weights).ok_or_else(invalid)?;
        for (entry, cost) in entries.iter_mut().zip(allocation) {
            entry.extra_cost = cost;
        }
    }
    // Physical confirmation/corrections are recorded now, not backdated to supplier issue.
    let today = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    sync(tx, a, c, "received", id, today, entries).await
}
pub(crate) async fn credit(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    id: i64,
    invoice: i64,
    d: &faktura_domain::credit_note::Data,
) -> Result<()> {
    let mut entries = vec![];
    for l in &d.lines {
        let Some(q) = l.return_quantity.as_deref() else {
            continue;
        };
        let q = quantity(q)?;
        let source=sqlx::query("SELECT catalog_id,-sum(quantity) AS quantity,max(item::text)::jsonb AS item FROM stock_movement WHERE company_id=$1 AND invoice_id=$2 AND source_line=$3 GROUP BY catalog_id")
            .bind(c).bind(invoice).bind(&l.id).fetch_optional(&mut **tx).await.map_err(err)?.ok_or_else(invalid)?;
        let used:Decimal=sqlx::query_scalar("SELECT COALESCE(sum(m.quantity),0) FROM stock_movement m JOIN credit_note n ON n.company_id=m.company_id AND n.id=m.credit_id WHERE m.company_id=$1 AND n.invoice_id=$2 AND m.source_line=$3")
            .bind(c).bind(invoice).bind(&l.id).fetch_one(&mut **tx).await.map_err(err)?;
        if used + q > source.get::<Decimal, _>("quantity") {
            return Err(invalid());
        }
        let item: serde_json::Value = source.get("item");
        entries.push(Entry {
            line: l.id.clone(),
            catalog: source.get::<i64, _>("catalog_id").to_string(),
            quantity: q,
            unit: item["unit"].as_str().ok_or_else(invalid)?.into(),
            acquisition_unit_cost: None,
            extra_cost: Decimal::ZERO,
        });
    }
    sync(tx, a, c, "credit", id, d.issued_on, entries).await
}

pub(crate) async fn retail(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    id: i64,
    d: &faktura_domain::invoice::DraftData,
    cancelled: bool,
) -> Result<()> {
    let entries = if cancelled {
        vec![]
    } else {
        d.calculation
            .lines
            .iter()
            .filter(|l| l.catalog_id.is_some())
            .map(|l| {
                Ok(Entry {
                    line: l.id.clone(),
                    catalog: l.catalog_id.clone().unwrap(),
                    quantity: -quantity(&l.quantity)?,
                    unit: l.unit.clone(),
                    acquisition_unit_cost: None,
                    extra_cost: Decimal::ZERO,
                })
            })
            .collect::<Result<Vec<_>>>()?
    };
    let today = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    sync(tx, a, c, "retail", id, today, entries).await
}

pub(crate) async fn validate_links(
    tx: &mut Transaction<'_, Postgres>,
    company: i64,
    calculation: &faktura_domain::invoice::Calculation,
) -> Result<()> {
    for line in &calculation.lines {
        if let Some(id) = &line.catalog_id {
            let id: i64 = id.parse().map_err(|_| invalid())?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM catalog WHERE company_id=$1 AND id=$2)",
            )
            .bind(company)
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
            if !exists {
                return Err(Error::NotFound);
            }
        }
    }
    Ok(())
}

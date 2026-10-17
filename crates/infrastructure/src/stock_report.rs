use async_trait::async_trait;
use faktura_application::stock_report::*;
use rust_decimal::{Decimal, RoundingStrategy};
use sqlx::{PgPool, Row};
use std::str::FromStr;
pub struct PgStockReport(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn invalid() -> Error {
    Error::Invalid(vec!["stock_valuation".into()])
}
fn id(s: &str) -> Result<i64> {
    s.parse().map_err(|_| Error::NotFound)
}
fn cost(r: &sqlx::postgres::PgRow) -> Cost {
    Cost {
        catalog_id: r.get::<i64, _>("catalog_id").to_string(),
        revision: r.get("revision"),
        effective_on: r.get("effective_on"),
        unit_cost: r
            .get::<Option<Decimal>, _>("unit_cost")
            .map(|v| v.normalize().to_string()),
        reason: r.get("reason"),
    }
}
#[async_trait]
impl StockReportStore for PgStockReport {
    async fn report(&self, a: &str, c: &str, q: Search) -> Result<Report> {
        let page = q.page.unwrap_or(1);
        if !(1..=400).contains(&page) || q.query.len() > 200 {
            return Err(invalid());
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        let date = q.as_of.unwrap_or(today);
        if date > today {
            return Err(invalid());
        }
        let rows=sqlx::query(r#"SELECT s.catalog_id,c.data,c.archived,
      COALESCE((SELECT sum(m.quantity) FROM stock_movement m WHERE m.company_id=s.company_id AND m.catalog_id=s.catalog_id AND m.booked_on<=$2),0) AS quantity,
      (SELECT unit_cost FROM stock_valuation v WHERE v.company_id=s.company_id AND v.catalog_id=s.catalog_id AND v.effective_on<=$2 ORDER BY effective_on DESC,revision DESC LIMIT 1) AS unit_cost,
      COALESCE((SELECT max(revision) FROM stock_valuation v WHERE v.company_id=s.company_id AND v.catalog_id=s.catalog_id),0) AS revision
      FROM stock_card s JOIN catalog c ON c.company_id=s.company_id AND c.id=s.catalog_id
      WHERE s.company_id=$1 AND s.opened_on<=$2 AND
       ($3='' OR strpos(lower(c.data->>'name'),lower($3))>0 OR strpos(lower(c.data->>'code'),lower($3))>0)
      ORDER BY lower(c.data->>'name'),s.catalog_id LIMIT 10001"#).bind(c).bind(date).bind(q.query.trim()).fetch_all(&mut *tx).await.map_err(err)?;
        if rows.len() > 10000 {
            return Err(invalid());
        }
        let mut automatic =
            std::collections::BTreeMap::<i64, faktura_domain::stock_cost::Valued>::new();
        if q.valuation == Valuation::Automatic {
            let ids: Vec<i64> = rows.iter().map(|r| r.get("catalog_id")).collect();
            let movements = sqlx::query("SELECT m.*,n.invoice_id AS credit_invoice_id FROM stock_movement m LEFT JOIN credit_note n ON n.company_id=m.company_id AND n.id=m.credit_id WHERE m.company_id=$1 AND m.catalog_id=ANY($2) AND m.booked_on<=$3 ORDER BY m.booked_on,m.id LIMIT 100001")
                .bind(c).bind(&ids).bind(date).fetch_all(&mut *tx).await.map_err(err)?;
            if movements.len() > 100000 {
                return Err(invalid());
            }
            use faktura_domain::stock_cost::{Event, Flow};
            let mut cards = std::collections::BTreeMap::<i64, Vec<Event>>::new();
            for m in movements {
                let line = m
                    .get::<Option<String>, _>("source_line")
                    .unwrap_or_default();
                let quantity: Decimal = m.get("quantity");
                let id: i64 = m.get("id");
                let flow = if let Some(source) = m.get::<Option<i64>, _>("received_id") {
                    Flow::Receipt {
                        source: format!("received:{source}:{line}"),
                        price: m.get("acquisition_unit_cost"),
                        total_quantity: m
                            .get::<Option<Decimal>, _>("acquisition_quantity")
                            .unwrap_or_default(),
                        extra: m.get("acquisition_extra_cost"),
                    }
                } else if let Some(source) = m.get::<Option<i64>, _>("credit_invoice_id") {
                    Flow::Return {
                        source: format!("invoice:{source}:{line}"),
                    }
                } else if let Some(source) = m.get::<Option<i64>, _>("invoice_id") {
                    Flow::Issue {
                        source: format!("invoice:{source}:{line}"),
                    }
                } else if let Some(source) = m.get::<Option<i64>, _>("retail_id") {
                    let source = format!("retail:{source}:{line}");
                    if quantity > Decimal::ZERO {
                        Flow::Return { source }
                    } else {
                        Flow::Issue { source }
                    }
                } else if let Some(movement) = m.get::<Option<i64>, _>("reverses_id") {
                    Flow::Reverse { movement }
                } else {
                    Flow::Unknown
                };
                cards
                    .entry(m.get("catalog_id"))
                    .or_default()
                    .push(Event { id, quantity, flow });
            }
            for (id, events) in cards {
                automatic.insert(id, faktura_domain::stock_cost::value(&events));
            }
        }
        let total = rows.len() as i64;
        let mut sum = Decimal::ZERO;
        let mut unvalued = 0;
        let mut negative = 0;
        let mut items = vec![];
        for r in rows {
            let quantity: Decimal = r.get("quantity");
            let auto = automatic.get(&r.get::<i64, _>("catalog_id"));
            let price: Option<Decimal> = if q.valuation == Valuation::Automatic {
                auto.and_then(|b| b.balance.unit_cost())
            } else {
                r.get("unit_cost")
            };
            let value = if quantity == Decimal::ZERO {
                Some(Decimal::ZERO)
            } else if quantity < Decimal::ZERO {
                negative += 1;
                None
            } else if q.valuation == Valuation::Automatic {
                auto.and_then(|b| b.balance.value)
                    .map(|v| v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero))
            } else {
                price.map(|p| {
                    (p * quantity).round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
                })
            };
            if let Some(v) = value {
                sum += v;
            } else {
                unvalued += 1;
            }
            let data: faktura_domain::catalog::CatalogData =
                serde_json::from_value(r.get("data")).map_err(err)?;
            let money = |v: Option<Decimal>| {
                v.map(|v| {
                    format!(
                        "{:.2}",
                        v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
                    )
                })
            };
            items.push(Item {
                issued_value: money(auto.and_then(|b| b.issued)),
                correction_stock_value: money(auto.and_then(|b| b.correction_stock)),
                correction_issued_value: money(auto.and_then(|b| b.correction_issued)),
                catalog_id: r.get::<i64, _>("catalog_id").to_string(),
                name: data.name,
                code: data.code,
                unit: data.unit,
                archived: r.get("archived"),
                quantity: quantity.normalize().to_string(),
                unit_cost: price.map(|p| p.normalize().to_string()),
                value: value.map(|v| format!("{v:.2}")),
                cost_revision: r.get("revision"),
            });
        }
        Ok(Report {
            as_of: date,
            items: items
                .into_iter()
                .skip(((page - 1) * 25) as usize)
                .take(25)
                .collect(),
            total,
            page,
            known_value: format!("{sum:.2}"),
            unvalued_count: unvalued,
            negative_count: negative,
        })
    }
    async fn set_cost(&self, a: &str, c: &str, cmd: CostCommand) -> Result<Cost> {
        uuid::Uuid::parse_str(&cmd.request_id).map_err(|_| invalid())?;
        if cmd.reason.trim().is_empty()
            || cmd.reason.len() > 500
            || cmd.reason.chars().any(char::is_control)
            || cmd.revision < 0
        {
            return Err(invalid());
        }
        let price = cmd
            .unit_cost
            .as_deref()
            .map(|v| {
                if v.is_empty()
                    || v.len() > 20
                    || !v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                {
                    return Err(invalid());
                }
                let p = Decimal::from_str(v).map_err(|_| invalid())?;
                if p < Decimal::ZERO || p > Decimal::from(1_000_000_000) || p.scale() > 4 {
                    return Err(invalid());
                }
                Ok(p)
            })
            .transpose()?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM stock_valuation WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != a || r.get::<serde_json::Value, _>("input") != input {
                return Err(Error::Conflict);
            }
            return Ok(cost(&r));
        }
        let i = id(&cmd.catalog_id)?;
        let opened: chrono::NaiveDate = sqlx::query_scalar(
            "SELECT opened_on FROM stock_card WHERE company_id=$1 AND catalog_id=$2 FOR UPDATE",
        )
        .bind(c)
        .bind(i)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        if cmd.effective_on < opened || cmd.effective_on > today {
            return Err(invalid());
        }
        let rev:i64=sqlx::query_scalar("SELECT COALESCE(max(revision),0) FROM stock_valuation WHERE company_id=$1 AND catalog_id=$2").bind(c).bind(i).fetch_one(&mut *tx).await.map_err(err)?;
        if rev != cmd.revision {
            return Err(Error::Conflict);
        }
        let row=sqlx::query("INSERT INTO stock_valuation(company_id,catalog_id,revision,effective_on,unit_cost,reason,actor_id,request_id,input) VALUES($1,$2,$3,$4,$5,$6,$7,$8::text::uuid,$9) RETURNING *").bind(c).bind(i).bind(rev+1).bind(cmd.effective_on).bind(price).bind(cmd.reason).bind(a).bind(cmd.request_id).bind(input).fetch_one(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(cost(&row))
    }
    async fn costs(&self, a: &str, c: &str, catalog: &str) -> Result<Vec<Cost>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let rows=sqlx::query("SELECT * FROM stock_valuation WHERE company_id=$1 AND catalog_id=$2 ORDER BY revision DESC LIMIT 100").bind(c).bind(id(catalog)?).fetch_all(&mut *tx).await.map_err(err)?;
        Ok(rows.iter().map(cost).collect())
    }
}

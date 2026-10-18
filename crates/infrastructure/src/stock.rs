use async_trait::async_trait;
use faktura_application::stock::*;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::str::FromStr;
pub struct PgStock(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn invalid() -> Error {
    Error::Invalid(vec!["stock".into()])
}
fn id(s: &str) -> Result<i64> {
    s.parse().map_err(|_| Error::NotFound)
}
fn quantity(s: &str) -> Result<Decimal> {
    if s.is_empty()
        || s.len() > 20
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-')
    {
        return Err(invalid());
    }
    let v = Decimal::from_str(s).map_err(|_| invalid())?;
    if v.scale() > 4 || v.abs() > Decimal::from(1000000) {
        return Err(invalid());
    }
    Ok(v)
}
fn number(v: Decimal) -> String {
    v.normalize().to_string()
}
fn map(r: &sqlx::postgres::PgRow) -> Result<Movement> {
    let source = [
        ("invoice", "invoice_id"),
        ("received", "received_id"),
        ("credit", "credit_id"),
        ("retail", "retail_id"),
    ]
    .into_iter()
    .find_map(|(kind, column)| {
        r.get::<Option<i64>, _>(column)
            .map(|id| (kind.to_string(), id.to_string()))
    });
    Ok(Movement {
        acquisition_extra_cost: Some(number(r.get("acquisition_extra_cost"))),
        acquisition_unit_cost: r
            .get::<Option<Decimal>, _>("acquisition_unit_cost")
            .map(number),
        acquisition_correction: r.get("acquisition_correction"),
        source_kind: source.as_ref().map(|v| v.0.clone()),
        source_id: source.map(|v| v.1),
        source_line: r.get("source_line"),
        id: r.get::<i64, _>("id").to_string(),
        catalog_id: r.get::<i64, _>("catalog_id").to_string(),
        quantity: number(r.get("quantity")),
        booked_on: r.get("booked_on"),
        kind: r.get("kind"),
        reason: r.get("reason"),
        item: serde_json::from_value(r.get("item")).map_err(err)?,
        reverses_id: r
            .get::<Option<i64>, _>("reverses_id")
            .map(|v| v.to_string()),
    })
}
async fn item(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    i: i64,
) -> Result<faktura_domain::catalog::CatalogData> {
    let value: serde_json::Value = sqlx::query_scalar(
        "SELECT data FROM catalog WHERE company_id=$1 AND id=$2 AND NOT archived FOR SHARE",
    )
    .bind(c)
    .bind(i)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    .ok_or(Error::NotFound)?;
    serde_json::from_value(value).map_err(err)
}
#[async_trait]
impl StockStore for PgStock {
    async fn cards(&self, a: &str, c: &str) -> Result<Vec<Card>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let rows=sqlx::query("SELECT s.*,c.data FROM stock_card s JOIN catalog c ON c.company_id=s.company_id AND c.id=s.catalog_id WHERE s.company_id=$1 ORDER BY lower(c.data->>'name'),s.catalog_id").bind(c).fetch_all(&mut *tx).await.map_err(err)?;
        rows.into_iter()
            .map(|r| {
                Ok(Card {
                    catalog_id: r.get::<i64, _>("catalog_id").to_string(),
                    revision: r.get("revision"),
                    opened_on: r.get("opened_on"),
                    quantity: number(r.get("quantity")),
                    item: serde_json::from_value(r.get("data")).map_err(err)?,
                })
            })
            .collect()
    }
    async fn movements(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        let page = q.page.unwrap_or(1);
        if !(1..=10000).contains(&page) || matches!((q.from,q.to),(Some(f),Some(t))if f>t) {
            return Err(invalid());
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let i = q.catalog_id.as_deref().map(id).transpose()?;
        let predicate="company_id=$1 AND ($2::bigint IS NULL OR catalog_id=$2) AND ($3::date IS NULL OR booked_on>=$3) AND ($4::date IS NULL OR booked_on<=$4)";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM stock_movement WHERE {predicate}"
        ))
        .bind(c)
        .bind(i)
        .bind(q.from)
        .bind(q.to)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let rows=sqlx::query(&format!("SELECT * FROM stock_movement WHERE {predicate} ORDER BY booked_on DESC,id DESC LIMIT 25 OFFSET $5")).bind(c).bind(i).bind(q.from).bind(q.to).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?;
        let summary = if let Some(i) = i {
            let row = sqlx::query(r#"
                SELECT
                  COALESCE(sum(m.quantity) FILTER (WHERE m.booked_on < $3),0) AS opening,
                  COALESCE(sum(m.quantity) FILTER (WHERE ($3::date IS NULL OR m.booked_on >= $3) AND m.kind IN ('manual','invoice','received','credit','retail') AND m.quantity>0),0) AS received,
                  COALESCE(-sum(m.quantity) FILTER (WHERE ($3::date IS NULL OR m.booked_on >= $3) AND m.kind IN ('manual','invoice','received','credit','retail') AND m.quantity<0),0) AS issued,
                  COALESCE(sum(m.quantity) FILTER (WHERE ($3::date IS NULL OR m.booked_on >= $3) AND m.kind NOT IN ('manual','invoice','received','credit','retail')),0) AS adjustments,
                  COALESCE(sum(m.quantity),0) AS closing
                FROM stock_card s LEFT JOIN stock_movement m
                  ON m.company_id=s.company_id AND m.catalog_id=s.catalog_id
                  AND ($4::date IS NULL OR m.booked_on <= $4)
                WHERE s.company_id=$1 AND s.catalog_id=$2
                  AND ($4::date IS NULL OR s.opened_on <= $4)
                GROUP BY s.catalog_id
            "#).bind(c).bind(i).bind(q.from).bind(q.to)
                .fetch_optional(&mut *tx).await.map_err(err)?;
            row.map(|r| Summary {
                opening: number(r.get("opening")),
                received: number(r.get("received")),
                issued: number(r.get("issued")),
                adjustments: number(r.get("adjustments")),
                closing: number(r.get("closing")),
            })
        } else {
            None
        };
        Ok(Page {
            summary,
            items: rows.iter().map(map).collect::<Result<_>>()?,
            total,
            page,
        })
    }
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Movement> {
        uuid::Uuid::parse_str(&cmd.request_id).map_err(|_| invalid())?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM stock_command WHERE company_id=$1 AND request_id=$2::text::uuid",
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
            return serde_json::from_value(r.get("result")).map_err(err);
        }
        let i = id(&cmd.catalog_id)?;
        let item = item(&mut tx, c, i).await?;
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        let card = sqlx::query(
            "SELECT * FROM stock_card WHERE company_id=$1 AND catalog_id=$2 FOR UPDATE",
        )
        .bind(c)
        .bind(i)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
        let (delta, date, kind, reason, reverses) = match cmd.action {
            Action::Open {
                quantity: q,
                booked_on,
                reason,
            } => {
                if card.is_some() {
                    return Err(Error::Conflict);
                }
                let q = quantity(&q)?;
                sqlx::query(
                    "INSERT INTO stock_card(company_id,catalog_id,opened_on) VALUES($1,$2,$3)",
                )
                .bind(c)
                .bind(i)
                .bind(booked_on)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
                (q, booked_on, "opening", reason, None)
            }
            action => {
                let card = card.as_ref().ok_or(Error::NotFound)?;
                match action {
                    Action::Move {
                        quantity: q,
                        booked_on,
                        reason,
                    } => {
                        let q = quantity(&q)?;
                        if q == Decimal::ZERO {
                            return Err(invalid());
                        }
                        (q, booked_on, "manual", reason, None)
                    }
                    Action::Count {
                        quantity: q,
                        revision,
                        reason,
                    } => {
                        let q = quantity(&q)?;
                        if q < Decimal::ZERO || card.get::<i64, _>("revision") != revision {
                            return Err(Error::Conflict);
                        }
                        (
                            q - card.get::<Decimal, _>("quantity"),
                            today,
                            "count",
                            reason,
                            None,
                        )
                    }
                    Action::Reverse {
                        movement_id,
                        reason,
                    } => {
                        let row=sqlx::query("SELECT * FROM stock_movement WHERE company_id=$1 AND catalog_id=$2 AND id=$3 FOR UPDATE").bind(c).bind(i).bind(id(&movement_id)?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
                        if !matches!(row.get::<String, _>("kind").as_str(), "manual" | "count") {
                            return Err(Error::Conflict);
                        }
                        let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stock_movement WHERE company_id=$1 AND reverses_id=$2)").bind(c).bind(id(&movement_id)?).fetch_one(&mut *tx).await.map_err(err)?;
                        if used {
                            return Err(Error::Conflict);
                        }
                        (
                            -row.get::<Decimal, _>("quantity"),
                            today,
                            "reversal",
                            reason,
                            Some(id(&movement_id)?),
                        )
                    }
                    _ => unreachable!(),
                }
            }
        };
        if reason.trim().is_empty()
            || reason.chars().count() > 500
            || reason.chars().any(char::is_control)
            || date > today
            || card
                .as_ref()
                .is_some_and(|r| date < r.get::<chrono::NaiveDate, _>("opened_on"))
        {
            return Err(invalid());
        }
        let cost = crate::stock_documents::acquisition_cost(cmd.acquisition_unit_cost.as_deref())?;
        if cost.is_some() && (delta <= Decimal::ZERO || reverses.is_some()) {
            return Err(invalid());
        }
        let balance = card
            .as_ref()
            .map(|r| r.get::<Decimal, _>("quantity"))
            .unwrap_or_default()
            + delta;
        if balance.abs() > Decimal::from(1000000000) || delta.abs() > Decimal::from(1000000) {
            return Err(invalid());
        }
        sqlx::query("UPDATE stock_card SET quantity=$3,revision=revision+1 WHERE company_id=$1 AND catalog_id=$2").bind(c).bind(i).bind(balance).execute(&mut *tx).await.map_err(err)?;
        let row=sqlx::query("INSERT INTO stock_movement(company_id,catalog_id,quantity,booked_on,kind,reason,item,reverses_id,actor_id,acquisition_unit_cost) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *").bind(c).bind(i).bind(delta).bind(date).bind(kind).bind(reason).bind(serde_json::to_value(item).map_err(err)?).bind(reverses).bind(a).bind(cost).fetch_one(&mut *tx).await.map_err(err)?;
        let result = map(&row)?;
        sqlx::query("INSERT INTO stock_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(cmd.request_id).bind(a).bind(input).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

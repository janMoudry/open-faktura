use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError, Result},
    purchase_report::*,
};
use faktura_domain::received::{money, Kind};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
#[derive(Clone)]
pub struct PgPurchaseReport(pub PgPool);
fn err(_: impl std::fmt::Display) -> CompanyError {
    CompanyError::Unavailable
}
#[async_trait]
impl PurchaseReportStore for PgPurchaseReport {
    async fn payments(&self, actor: &str, company: &str, q: Search) -> Result<Page> {
        q.validate()?;
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, actor, company, false).await?;
        let source = "FROM received_invoice d CROSS JOIN LATERAL jsonb_array_elements(d.payments) p WHERE d.company_id=$1 AND (p->>'paid_on')::date BETWEEN $2 AND $3 AND ($4='' OR position(lower($4) in lower(d.number||' '||(d.data->'supplier'->>'name')||' '||(d.data->>'description')||' '||coalesce(p->>'note','')))>0) AND ($5 IN ('','all') OR p->>'method'=$5) AND CASE $6 WHEN 'all' THEN true WHEN 'reversed' THEN p->>'reversed_at' IS NOT NULL ELSE p->>'reversed_at' IS NULL END";
        let total: i64 = sqlx::query_scalar(&format!("SELECT count(*) {source}"))
            .bind(c)
            .bind(q.from)
            .bind(q.to)
            .bind(&q.query)
            .bind(&q.method)
            .bind(&q.status)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        let rows = sqlx::query(&format!("SELECT d.id,d.number,d.kind,d.data,p AS payment {source} ORDER BY (p->>'paid_on')::date DESC,d.id DESC,p->>'id' DESC LIMIT 25 OFFSET $7"))
            .bind(c).bind(q.from).bind(q.to).bind(&q.query).bind(&q.method).bind(&q.status).bind((q.page.unwrap_or(1)-1)*25)
            .fetch_all(&mut *tx).await.map_err(err)?;
        let items = rows
            .into_iter()
            .map(|r| {
                let data: faktura_domain::received::Data =
                    serde_json::from_value(r.get("data")).map_err(err)?;
                Ok(Entry {
                    document_id: r.get::<i64, _>("id").to_string(),
                    number: r.get("number"),
                    kind: if r.get::<String, _>("kind") == "receipt" {
                        Kind::Receipt
                    } else {
                        Kind::Invoice
                    },
                    currency: data.currency.clone(),
                    supplier: data.supplier.name,
                    description: if data.description.is_empty() {
                        data.lines
                            .first()
                            .map(|l| l.description.clone())
                            .unwrap_or_default()
                    } else {
                        data.description
                    },
                    payment: serde_json::from_value(r.get("payment")).map_err(err)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Page {
            items,
            total,
            page: q.page.unwrap_or(1),
        })
    }
    async fn overview(&self, actor: &str, company: &str, p: Period) -> Result<Overview> {
        p.validate()?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, actor, company, false).await?;
        let r = sqlx::query("WITH docs AS (SELECT * FROM received_invoice WHERE company_id=$1) SELECT
            coalesce((SELECT sum(report_czk(total,data->>'currency',data->'exchange_rate')) FROM docs WHERE NOT cancelled AND (data->>'issued_on')::date BETWEEN $2 AND $3),0) AS purchases,
            coalesce((SELECT sum(report_czk((p->>'amount')::numeric,data->>'currency',data->'exchange_rate')) FROM docs CROSS JOIN LATERAL jsonb_array_elements(payments) p WHERE p->>'reversed_at' IS NULL AND (p->>'paid_on')::date BETWEEN $2 AND $3),0) AS expenses,
            coalesce((SELECT sum(report_czk(total-paid,data->>'currency',data->'exchange_rate')) FROM docs WHERE NOT cancelled AND kind='invoice'),0) AS outstanding,
            coalesce((SELECT sum(report_czk(total-paid,data->>'currency',data->'exchange_rate')) FROM docs WHERE NOT cancelled AND kind='invoice' AND due_on<(now() AT TIME ZONE 'Europe/Prague')::date),0) AS overdue")
            .bind(c).bind(p.from).bind(p.to).fetch_one(&mut *tx).await.map_err(err)?;
        Ok(Overview {
            purchases: money(r.get::<Decimal, _>("purchases")),
            expenses: money(r.get::<Decimal, _>("expenses")),
            outstanding: money(r.get::<Decimal, _>("outstanding")),
            overdue: money(r.get::<Decimal, _>("overdue")),
        })
    }
}

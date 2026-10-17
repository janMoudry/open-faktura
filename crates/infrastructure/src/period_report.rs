use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError, Result},
    period_report::*,
};
use faktura_domain::received::money;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
pub struct PgPeriodReport(pub PgPool);
fn err(_: impl std::fmt::Display) -> CompanyError {
    CompanyError::Unavailable
}
const SOURCE: &str = r#"WITH sources AS (
 SELECT id,'invoice'::text kind,number,(CASE WHEN data#>>'{calculation,tax_mode}'='vat_payer' THEN coalesce(data->>'tax_on',data->>'issued_on') ELSE data->>'issued_on' END)::date issued_on,
 CASE WHEN $4 THEN (totals#>>'{amounts,net}')::numeric+coalesce((SELECT sum((receipt#>>'{totals,amounts,net}')::numeric) FROM jsonb_array_elements(advances->'sources') src CROSS JOIN LATERAL jsonb_array_elements(src->'tax_receipts') receipt),0) ELSE (totals->>'payable')::numeric+coalesce((advances->>'credited')::numeric,0)-coalesce((advances->>'rounding_adjustment')::numeric,0) END amount,'sales'::text side
 FROM invoice_draft WHERE company_id=$1 AND number IS NOT NULL AND totals->>'currency'='CZK'
 UNION ALL SELECT id,'credit_note',number,(CASE WHEN source#>>'{data,calculation,tax_mode}'='vat_payer' THEN coalesce(data->>'tax_on',data->>'issued_on') ELSE data->>'issued_on' END)::date,CASE WHEN $4 THEN (totals#>>'{amounts,net}')::numeric ELSE (totals->>'payable')::numeric END,'sales'
 FROM credit_note WHERE company_id=$1 AND status='issued' AND totals->>'currency'='CZK'
 UNION ALL SELECT id,'retail',number,(data->'invoice'->>'issued_on')::date,CASE WHEN $4 THEN (totals#>>'{amounts,net}')::numeric ELSE (totals->>'payable')::numeric END,'sales'
 FROM retail_document WHERE company_id=$1 AND NOT cancelled
 UNION ALL SELECT id,CASE kind WHEN 'receipt' THEN 'purchase_receipt' ELSE 'received' END,number,CASE kind WHEN 'receipt' THEN (data->>'issued_on')::date ELSE received_on END,CASE WHEN $4 THEN (SELECT sum((line->>'net')::numeric) FROM jsonb_array_elements(data->'lines') line) ELSE total END,'purchases'
 FROM received_invoice WHERE company_id=$1 AND NOT cancelled
), docs AS (SELECT * FROM sources WHERE issued_on BETWEEN $2 AND $3) "#;
#[async_trait]
impl PeriodReportStore for PgPeriodReport {
    async fn report(&self, a: &str, c: &str, q: Search) -> Result<Report> {
        q.validate()?;
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let mut report = load(&mut tx, c, &q).await?;
        if q.compare_previous {
            let (from, to) = q.previous_period();
            let previous = load(
                &mut tx,
                c,
                &Search {
                    from,
                    to,
                    page: Some(1),
                    value_mode: q.value_mode,
                    cumulative: q.cumulative,
                    compare_previous: false,
                },
            )
            .await?;
            let difference = |current: &str, previous: &str| -> Result<String> {
                Ok(money(
                    current.parse::<Decimal>().map_err(err)?
                        - previous.parse::<Decimal>().map_err(err)?,
                ))
            };
            report.comparison = Some(Comparison {
                from,
                to,
                sales_difference: difference(&report.sales, &previous.sales)?,
                purchases_difference: difference(&report.purchases, &previous.purchases)?,
                sales: previous.sales,
                purchases: previous.purchases,
                months: previous.months,
            });
        }
        Ok(report)
    }
}
async fn load(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    c: i64,
    q: &Search,
) -> Result<Report> {
    let r=sqlx::query(&format!("{SOURCE} SELECT count(*) total,coalesce(sum(amount) FILTER (WHERE side='sales'),0) sales,coalesce(sum(amount) FILTER (WHERE side='purchases'),0) purchases FROM docs"))
 .bind(c).bind(q.from).bind(q.to).bind(q.value_mode == ValueMode::Net).fetch_one(&mut **tx).await.map_err(err)?;
    let rows=sqlx::query(&format!("{SOURCE} SELECT to_char(m,'YYYY-MM') AS month,coalesce(sum(amount) FILTER (WHERE side='sales'),0) sales,coalesce(sum(amount) FILTER (WHERE side='purchases'),0) purchases FROM generate_series(date_trunc('month',$2::date),date_trunc('month',$3::date),interval '1 month') m LEFT JOIN docs ON date_trunc('month',issued_on)=m GROUP BY m ORDER BY m"))
 .bind(c).bind(q.from).bind(q.to).bind(q.value_mode == ValueMode::Net).fetch_all(&mut **tx).await.map_err(err)?;
    let (mut sales, mut purchases) = (Decimal::ZERO, Decimal::ZERO);
    let months = rows
        .into_iter()
        .map(|r| {
            let current_sales = r.get::<Decimal, _>("sales");
            let current_purchases = r.get::<Decimal, _>("purchases");
            sales += current_sales;
            purchases += current_purchases;
            Month {
                month: r.get("month"),
                sales: money(if q.cumulative { sales } else { current_sales }),
                purchases: money(if q.cumulative {
                    purchases
                } else {
                    current_purchases
                }),
            }
        })
        .collect();
    let rows = sqlx::query(&format!(
        "{SOURCE} SELECT * FROM docs ORDER BY issued_on DESC,kind,id DESC LIMIT 25 OFFSET $5"
    ))
    .bind(c)
    .bind(q.from)
    .bind(q.to)
    .bind(q.value_mode == ValueMode::Net)
    .bind((q.page.unwrap_or(1) - 1) * 25)
    .fetch_all(&mut **tx)
    .await
    .map_err(err)?;
    let items = rows
        .into_iter()
        .map(|r| Entry {
            id: r.get::<i64, _>("id").to_string(),
            kind: r.get("kind"),
            number: r.get("number"),
            reported_on: r.get("issued_on"),
            amount: money(r.get::<Decimal, _>("amount")),
            side: r.get("side"),
        })
        .collect();
    Ok(Report {
        sales: money(r.get::<Decimal, _>("sales")),
        purchases: money(r.get::<Decimal, _>("purchases")),
        currency: "CZK".into(),
        total: r.get("total"),
        page: q.page.unwrap_or(1),
        months,
        items,
        comparison: None,
    })
}

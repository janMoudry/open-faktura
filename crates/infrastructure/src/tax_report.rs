use async_trait::async_trait;
use chrono::Datelike;
use faktura_application::{
    company::{CompanyError, Result},
    tax_report::*,
};
use faktura_domain::received::money;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
pub struct PgTaxReport(pub PgPool);
fn err(_: impl std::fmt::Display) -> CompanyError {
    CompanyError::Unavailable
}
// Use stored VAT after advance deductions. DDPP are then counted exactly once.
pub(crate) const CASH: &str = r#"SELECT m.id,'cash'::text kind,m.number,(m.tax#>>'{tax,tax_on}')::date reported_on,
 (m.tax#>>'{amounts,net}')::numeric net,(m.tax#>>'{amounts,vat}')::numeric vat,
 CASE WHEN m.amount>0 THEN 'sales' ELSE 'purchases' END side
 FROM cash_movement m JOIN cash_account a ON a.company_id=m.company_id AND a.id=m.account_id
 WHERE m.company_id=$1 AND NOT m.cancelled AND NOT m.automatic AND m.tax IS NOT NULL AND a.currency='CZK'
 AND NOT EXISTS(SELECT 1 FROM cash_match x WHERE x.company_id=m.company_id AND x.movement_id=m.id AND x.detached_at IS NULL)"#;
pub(crate) const TAX: &str = r#"sources AS (
 SELECT id,'invoice'::text kind,number,(data->>'tax_on')::date reported_on,report_czk((totals#>>'{amounts,vat}')::numeric,totals->>'currency',data#>'{calculation,exchange_rate}') amount,'sales'::text side
 FROM invoice_draft WHERE company_id=$1 AND number IS NOT NULL AND data#>>'{calculation,tax_mode}'='vat_payer'
 UNION ALL SELECT id,'credit_note',number,(data->>'tax_on')::date,report_czk((totals#>>'{amounts,vat}')::numeric,totals->>'currency',source#>'{data,calculation,exchange_rate}'),'sales' FROM credit_note WHERE company_id=$1 AND status='issued' AND source#>>'{data,calculation,tax_mode}'='vat_payer'
 UNION ALL SELECT id,'tax_receipt',number,(snapshot#>>'{data,issued_on}')::date,report_czk((snapshot#>>'{totals,amounts,vat}')::numeric,snapshot#>>'{totals,currency}',snapshot#>'{data,calculation,exchange_rate}'),'sales' FROM tax_receipt WHERE company_id=$1
 UNION ALL SELECT id,'retail',number,(data#>>'{invoice,issued_on}')::date,(totals#>>'{amounts,vat}')::numeric,'sales' FROM retail_document WHERE company_id=$1 AND NOT cancelled AND totals->>'currency'='CZK'
 UNION ALL SELECT id,CASE kind WHEN 'receipt' THEN 'purchase_receipt' ELSE 'received' END,number,coalesce((data->>'tax_on')::date,CASE kind WHEN 'receipt' THEN (data->>'issued_on')::date ELSE received_on END),report_czk((SELECT sum((l->>'vat')::numeric) FROM jsonb_array_elements(data->'lines') l),coalesce(data->>'currency','CZK'),data->'exchange_rate'),'purchases' FROM received_invoice WHERE company_id=$1 AND NOT cancelled
 UNION ALL SELECT id,kind,number,reported_on,vat,side FROM cash
), docs AS (SELECT * FROM sources WHERE reported_on BETWEEN $2 AND $3 AND amount IS NOT NULL) "#;
impl PgTaxReport {
    async fn report_inner(&self, a: &str, c: &str, q: Search, all: bool) -> Result<Report> {
        let mut tx = self
            .0
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ")
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let r=sqlx::query("SELECT (now() AT TIME ZONE (settings->>'timezone'))::date today,settings->>'tax_mode' tax_mode FROM company WHERE id=$1").bind(c).fetch_one(&mut *tx).await.map_err(err)?;
        let today: chrono::NaiveDate = r.get("today");
        let year = q.year.unwrap_or(today.year());
        let page = q.page.unwrap_or(1);
        if !(1900..=today.year()).contains(&year) || !(1..=10000).contains(&page) {
            return Err(CompanyError::Invalid(vec!["period".into()]));
        }
        let from = chrono::NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let to = chrono::NaiveDate::from_ymd_opt(year, 12, 31)
            .unwrap()
            .min(today);
        let tax_source = format!("WITH cash AS ({CASH}), {TAX}");
        let rows=sqlx::query(&format!("{tax_source} SELECT coalesce(sum(amount) FILTER(WHERE side='sales'),0) output,coalesce(sum(amount) FILTER(WHERE side='purchases'),0) input FROM docs")).bind(c).bind(from).bind(to).fetch_one(&mut *tx).await.map_err(err)?;
        let output: Decimal = rows.get("output");
        let input: Decimal = rows.get("input");
        let (unit, step, label) = if q.quarterly {
            ("quarter", "3 months", "YYYY-\"Q\"Q")
        } else {
            ("month", "1 month", "YYYY-MM")
        };
        let rows=sqlx::query(&format!("{tax_source} SELECT to_char(m,'{label}') period,coalesce(sum(amount) FILTER(WHERE side='sales'),0) output,coalesce(sum(amount) FILTER(WHERE side='purchases'),0) input FROM generate_series(date_trunc('{unit}',$2::date),date_trunc('{unit}',$3::date),interval '{step}') m LEFT JOIN docs ON date_trunc('{unit}',reported_on)=m GROUP BY m ORDER BY m")).bind(c).bind(from).bind(to).fetch_all(&mut *tx).await.map_err(err)?;
        let periods = rows
            .into_iter()
            .map(|r| {
                let output: Decimal = r.get("output");
                let input: Decimal = r.get("input");
                Period {
                    period: r.get("period"),
                    output: money(output),
                    input: money(input),
                    difference: money(output - input),
                }
            })
            .collect();
        let turnover_source=format!("{} SELECT id,kind,number,issued_on reported_on,amount,side FROM docs WHERE side='sales' UNION ALL SELECT id,kind,number,reported_on,net,side FROM ({CASH}) cash WHERE side='sales' AND reported_on BETWEEN $2 AND $3",crate::period_report::SOURCE);
        let turnover: Decimal = sqlx::query_scalar(&format!(
            "SELECT coalesce(sum(amount),0) FROM ({turnover_source}) t"
        ))
        .bind(c)
        .bind(from)
        .bind(to)
        .bind(true)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let source = if q.turnover_sources {
            turnover_source
        } else {
            format!("{tax_source} SELECT * FROM docs")
        };
        let count_sql = format!("SELECT count(*) FROM ({source}) t");
        let mut count = sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(c)
            .bind(from)
            .bind(to);
        if q.turnover_sources {
            count = count.bind(true);
        }
        let total = count.fetch_one(&mut *tx).await.map_err(err)?;
        if all && total > 10000 {
            return Err(CompanyError::Invalid(vec!["export_limit".into()]));
        }
        let offset = if q.turnover_sources { "$5" } else { "$4" };
        let limit = if all { 10000 } else { 25 };
        let item_sql=format!("SELECT * FROM ({source}) t ORDER BY reported_on DESC,kind,id DESC LIMIT {limit} OFFSET {offset}");
        let mut query = sqlx::query(&item_sql).bind(c).bind(from).bind(to);
        if q.turnover_sources {
            query = query.bind(true);
        }
        let items = query
            .bind((page - 1) * 25)
            .fetch_all(&mut *tx)
            .await
            .map_err(err)?
            .into_iter()
            .map(|r| Entry {
                id: r.get::<i64, _>("id").to_string(),
                kind: r.get("kind"),
                number: r.get("number"),
                reported_on: r.get("reported_on"),
                amount: money(r.get("amount")),
                side: r.get("side"),
            })
            .collect();
        tx.commit().await.map_err(err)?;
        Ok(Report {
            year,
            tax_mode: r.get("tax_mode"),
            turnover: money(turnover),
            output: money(output),
            input: money(input),
            difference: money(output - input),
            periods,
            items,
            total,
            page,
        })
    }
}

#[async_trait]
impl TaxReportStore for PgTaxReport {
    async fn report(&self, a: &str, c: &str, q: Search) -> Result<Report> {
        self.report_inner(a, c, q, false).await
    }
}
#[async_trait]
impl TaxExportStore for PgTaxReport {
    async fn export(
        &self,
        a: &str,
        c: &str,
        mut request: ExportRequest,
    ) -> Result<faktura_application::export::File> {
        request.search.page = Some(1);
        let turnover = request.search.turnover_sources;
        let report = self.report_inner(a, c, request.search, true).await?;
        let cs = matches!(request.language, faktura_application::export::Language::Cs);
        let mut rows = vec![if cs {
            vec![
                "Typ zdroje",
                "ID zdroje",
                "Číslo dokladu",
                "Rozhodné datum",
                "Směr",
                "Částka CZK",
                "Rozsah",
            ]
        } else {
            vec![
                "Source kind",
                "Source ID",
                "Document number",
                "Report date",
                "Side",
                "Amount CZK",
                "Scope",
            ]
        }
        .into_iter()
        .map(str::to_owned)
        .collect()];
        let scope = if turnover {
            if cs {
                "Orientační obrat; nejde o zákonný obrat"
            } else {
                "Indicative turnover; not statutory turnover"
            }
        } else if cs {
            "Evidovaná DPH; nejde o přiznání ani uznaný nárok na odpočet"
        } else {
            "Recorded VAT; not a tax return or validated deduction"
        };
        for e in report.items {
            rows.push(vec![
                e.kind,
                e.id,
                e.number,
                e.reported_on.to_string(),
                e.side,
                e.amount,
                scope.into(),
            ]);
        }
        let (bytes, extension, media_type) = crate::export::encode(&rows, request.format)?;
        Ok(faktura_application::export::File {
            filename: format!(
                "{}-{}.{}",
                if turnover {
                    "turnover-evidence"
                } else {
                    "vat-evidence"
                },
                report.year,
                extension
            ),
            media_type,
            bytes,
        })
    }
}

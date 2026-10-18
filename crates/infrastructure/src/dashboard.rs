use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError, Result},
    dashboard::*,
};
use faktura_domain::received::money;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
pub struct PgDashboard(pub PgPool);
fn err(_: impl std::fmt::Display) -> CompanyError {
    CompanyError::Unavailable
}
// The existing report source supplies signed document values and excludes drafts/cancellations.
const FINANCE: &str = r#",
finance AS (
 SELECT d.*,coalesce(s.due_on,r.due_on) due_on,
 CASE d.kind WHEN 'invoice' THEN report_czk(s.outstanding,i.totals->>'currency',i.data#>'{calculation,exchange_rate}') WHEN 'received' THEN report_czk(greatest(r.total-r.paid,0),coalesce(r.data->>'currency','CZK'),r.data->'exchange_rate')
 WHEN 'credit_note' THEN d.amount+report_czk(coalesce((SELECT sum(e.amount) FROM credit_note_event e WHERE e.company_id=$1 AND e.credit_id=d.id AND e.kind IN ('offset','refund') AND NOT EXISTS(SELECT 1 FROM credit_note_event x WHERE x.company_id=e.company_id AND x.credit_id=e.credit_id AND x.reverses=e.id)),0),n.totals->>'currency',n.source#>'{data,calculation,exchange_rate}')
 ELSE 0 END remaining,
 i.data->>'contact_id' contact_id,i.data->'customer' customer
 FROM docs d
 LEFT JOIN credit_note n ON d.kind='credit_note' AND n.company_id=$1 AND n.id=d.id
 LEFT JOIN invoice_draft i ON d.kind='invoice' AND i.company_id=$1 AND i.id=d.id
 LEFT JOIN invoice_settlement s ON d.kind='invoice' AND s.company_id=$1 AND s.invoice_id=d.id
 LEFT JOIN received_invoice r ON d.kind='received' AND r.company_id=$1 AND r.id=d.id
),
partners AS (
 SELECT id,kind,issued_on,due_on,amount,remaining,contact_id,customer FROM finance WHERE kind='invoice'
 UNION ALL
 SELECT id,'advance',(data#>>'{invoice,issued_on}')::date,(data#>>'{invoice,due_on}')::date,
 report_czk((advance_totals->>'payable')::numeric,advance_totals->>'currency',data#>'{invoice,calculation,exchange_rate}'),report_czk(greatest((advance_totals->>'payable')::numeric-advance_paid,0),advance_totals->>'currency',data#>'{invoice,calculation,exchange_rate}'),
 data#>>'{invoice,contact_id}',data#>'{invoice,customer}'
 FROM sales_document WHERE company_id=$1 AND kind='advance' AND NOT archived
 AND (data#>>'{invoice,issued_on}')::date <= $3
),
named AS (
 SELECT *,CASE WHEN contact_id IS NOT NULL THEN 'contact:'||contact_id
 ELSE kind||':'||id END identity FROM partners
)
"#;
#[async_trait]
impl DashboardStore for PgDashboard {
    async fn dashboard(&self, a: &str, c: &str, q: Search) -> Result<Dashboard> {
        let mut tx = self
            .0
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ")
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let today: chrono::NaiveDate = sqlx::query_scalar(
            "SELECT (now() AT TIME ZONE (settings->>'timezone'))::date FROM company WHERE id=$1",
        )
        .bind(c)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        use chrono::Datelike;
        let year = q.year.unwrap_or(today.year());
        if !(1900..=today.year()).contains(&year) {
            return Err(CompanyError::Invalid(vec!["year".into()]));
        }
        let start = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
        let source = format!("{}{}", crate::period_report::SOURCE, FINANCE);
        let mut sales = Balance::default();
        let mut purchases = Balance::default();
        // Settlement amounts always use gross values, including cash and non-cash settlement.
        for net in [false, true] {
            let sql=format!("{source} SELECT side,coalesce(sum(amount),0) total,coalesce(sum(remaining),0) remaining FROM finance WHERE extract(year FROM issued_on)=$5 GROUP BY side");
            let rows = sqlx::query(&sql)
                .bind(c)
                .bind(start)
                .bind(today)
                .bind(net)
                .bind(year)
                .fetch_all(&mut *tx)
                .await
                .map_err(err)?;
            for balance in [&mut sales, &mut purchases] {
                if net {
                    balance.net = "0.00".into()
                } else {
                    balance.gross = "0.00".into();
                    balance.settled = "0.00".into();
                    balance.outstanding = "0.00".into();
                }
            }
            for r in rows {
                let b = if r.get::<String, _>("side") == "sales" {
                    &mut sales
                } else {
                    &mut purchases
                };
                let total = r.get::<Decimal, _>("total");
                if net {
                    b.net = money(total)
                } else {
                    let remaining = r.get::<Decimal, _>("remaining");
                    b.gross = money(total);
                    b.outstanding = money(remaining);
                    b.settled = money(total - remaining);
                }
            }
        }
        let sql=format!("{source} SELECT side,CASE WHEN due_on >= $3 THEN 'not_due' WHEN $3-due_on <=30 THEN 'days_1_30' WHEN $3-due_on<=60 THEN 'days_31_60' WHEN $3-due_on<=90 THEN 'days_61_90' ELSE 'days_91_plus' END bucket,sum(remaining) amount,count(*) count FROM finance WHERE kind IN ('invoice','received') AND remaining>0 AND ($5 OR extract(year FROM issued_on)=$6) GROUP BY side,bucket ORDER BY side,bucket");
        let rows = sqlx::query(&sql)
            .bind(c)
            .bind(start)
            .bind(today)
            .bind(false)
            .bind(q.all_years)
            .bind(year)
            .fetch_all(&mut *tx)
            .await
            .map_err(err)?;
        let mut aging = Vec::new();
        for side in ["sales", "purchases"] {
            for bucket in [
                "not_due",
                "days_1_30",
                "days_31_60",
                "days_61_90",
                "days_91_plus",
            ] {
                let row = rows.iter().find(|r| {
                    r.get::<String, _>("side") == side && r.get::<String, _>("bucket") == bucket
                });
                aging.push(Age {
                    side: side.into(),
                    bucket: bucket.into(),
                    amount: row
                        .map(|r| money(r.get("amount")))
                        .unwrap_or_else(|| "0.00".into()),
                    count: row.map(|r| r.get("count")).unwrap_or(0),
                });
            }
        }
        let mut ranks = Vec::new();
        for debt in [false, true] {
            let (amount, filter) = if debt {
                ("remaining", "remaining>0 AND due_on<$3")
            } else {
                ("amount", "amount>0")
            };
            let sql=format!("{source} SELECT contact_id,coalesce((array_agg(customer->>'name' ORDER BY issued_on DESC,id DESC))[1],'') name,sum({amount}) amount,count(*) count FROM named WHERE {filter} GROUP BY identity,contact_id ORDER BY sum({amount}) DESC,identity LIMIT 3");
            let rows = sqlx::query(&sql)
                .bind(c)
                .bind(start)
                .bind(today)
                .bind(false)
                .fetch_all(&mut *tx)
                .await
                .map_err(err)?;
            ranks.push(
                rows.into_iter()
                    .map(|r| Partner {
                        contact_id: r.get("contact_id"),
                        name: r.get("name"),
                        amount: money(r.get("amount")),
                        count: r.get("count"),
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let debtors = ranks.pop().unwrap();
        let customers = ranks.pop().unwrap();
        tx.commit().await.map_err(err)?;
        Ok(Dashboard {
            today: today.to_string(),
            year,
            sales,
            purchases,
            aging,
            customers,
            debtors,
        })
    }
}

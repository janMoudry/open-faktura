use async_trait::async_trait;
use faktura_application::{
    export::{File, Language},
    item_sales::*,
};
use faktura_domain::received::money;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
pub struct PgItemSales(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
const FILTER: &str = "FROM item_sales_line WHERE company_id=$1 AND ($2::date IS NULL OR issued_on>=$2) AND ($3::date IS NULL OR issued_on<=$3) AND ($4::text IS NULL OR kind=$4) AND ($5::text IS NULL OR catalog_id::text=$5) AND (strpos(lower(name),lower($6))>0 OR strpos(lower(coalesce(code,'')),lower($6))>0)";
impl PgItemSales {
    async fn load(&self, a: &str, c: &str, q: Search, all: bool) -> Result<Report> {
        q.validate()?;
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        macro_rules! query {
            ($sql:expr) => {
                sqlx::query($sql)
                    .bind(c)
                    .bind(q.from)
                    .bind(q.to)
                    .bind(&q.kind)
                    .bind(&q.catalog_id)
                    .bind(q.query.trim())
            };
        }
        let sql=format!("SELECT catalog_id,name,coalesce(code,'') code,unit,currency,sum(quantity) quantity,sum(net) net,sum(gross) gross {FILTER} GROUP BY catalog_id,name,code,unit,currency ORDER BY lower(name),catalog_id,unit,currency LIMIT 10001");
        let rows = query!(&sql).fetch_all(&mut *tx).await.map_err(err)?;
        if rows.len() > 10000 {
            return Err(Error::Invalid(vec!["report_limit".into()]));
        }
        let total = rows.len() as i64;
        let mut totals = BTreeMap::<String, (Decimal, Decimal)>::new();
        let items = rows
            .into_iter()
            .map(|r| {
                let currency: String = r.get("currency");
                let n: Decimal = r.get("net");
                let g: Decimal = r.get("gross");
                let sum = totals.entry(currency.clone()).or_default();
                sum.0 += n;
                sum.1 += g;
                Item {
                    catalog_id: r.get::<i64, _>("catalog_id").to_string(),
                    name: r.get("name"),
                    code: r.get("code"),
                    unit: r.get("unit"),
                    currency,
                    quantity: r.get::<Decimal, _>("quantity").normalize().to_string(),
                    net: money(n),
                    gross: money(g),
                }
            })
            .collect::<Vec<_>>();
        let page = q.page.unwrap_or(1);
        let source_page = q.source_page.unwrap_or(1);
        let mut sources = Vec::new();
        let mut source_total = 0;
        if q.catalog_id.is_some() && !all {
            source_total = query!(&format!("SELECT count(*) count {FILTER}"))
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?
                .get("count");
            let sql=format!("SELECT * {FILTER} ORDER BY issued_on DESC,kind,document_id DESC,line_id LIMIT 25 OFFSET $7");
            sources = query!(&sql)
                .bind((source_page - 1) * 25)
                .fetch_all(&mut *tx)
                .await
                .map_err(err)?
                .into_iter()
                .map(|r| Source {
                    document_id: r.get::<i64, _>("document_id").to_string(),
                    number: r.get("number"),
                    kind: r.get("kind"),
                    issued_on: r.get("issued_on"),
                    description: r.get("description"),
                    unit: r.get("unit"),
                    currency: r.get("currency"),
                    quantity: r.get::<Decimal, _>("quantity").normalize().to_string(),
                    net: money(r.get("net")),
                    gross: money(r.get("gross")),
                })
                .collect();
        }
        Ok(Report {
            items: if all {
                items
            } else {
                items
                    .into_iter()
                    .skip(((page - 1) * 25) as usize)
                    .take(25)
                    .collect()
            },
            totals: totals
                .into_iter()
                .map(|(currency, (n, g))| Total {
                    currency,
                    net: money(n),
                    gross: money(g),
                })
                .collect(),
            total,
            page,
            sources,
            source_total,
            source_page,
        })
    }
}
#[async_trait]
impl ItemSalesStore for PgItemSales {
    async fn report(&self, a: &str, c: &str, q: Search) -> Result<Report> {
        self.load(a, c, q, false).await
    }
    async fn export(&self, a: &str, c: &str, q: ExportRequest) -> Result<File> {
        let r = self.load(a, c, q.search, true).await?;
        let cs = matches!(q.language, Language::Cs);
        let labels = if cs {
            [
                "Položka",
                "Kód",
                "Jednotka",
                "Měna",
                "Prodané množství",
                "Bez DPH",
                "Včetně DPH",
            ]
        } else {
            [
                "Item",
                "Code",
                "Unit",
                "Currency",
                "Quantity sold",
                "Net amount",
                "Gross amount",
            ]
        };
        let mut rows = vec![labels.into_iter().map(str::to_string).collect()];
        for i in r.items {
            let amount = |s: String| if cs { s.replace('.', ",") } else { s };
            rows.push(vec![
                i.name,
                i.code,
                i.unit,
                i.currency,
                amount(i.quantity),
                amount(i.net),
                amount(i.gross),
            ]);
        }
        let (bytes, media_type, extension) = crate::export::encode(&rows, q.format)?;
        Ok(File {
            bytes,
            media_type,
            filename: format!("item-sales.{extension}"),
        })
    }
}

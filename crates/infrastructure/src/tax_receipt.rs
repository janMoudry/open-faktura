use crate::draft::authorize;
use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    tax_receipt::*,
};
use faktura_domain::invoice::{Amounts, Line, LineResult, PriceMode, Rounding, TaxMode};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::str::FromStr;
#[derive(Clone)]
pub struct PgTaxReceipt(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn map(r: sqlx::postgres::PgRow) -> Result<Document> {
    serde_json::from_value(r.get("snapshot")).map_err(err)
}
async fn source(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    d: &str,
    p: &str,
    lock: bool,
) -> Result<(Document, i64)> {
    let r = sqlx::query(&format!(
        "SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 AND kind='advance' {}",
        if lock { "FOR UPDATE" } else { "" }
    ))
    .bind(c)
    .bind(id(d)?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    .ok_or(Error::NotFound)?;
    if r.get::<bool, _>("archived") {
        return Err(Error::Conflict);
    }
    let payment=sqlx::query("SELECT amount::text,paid_on,method,reversed_at IS NOT NULL AS reversed FROM advance_payment WHERE company_id=$1 AND document_id=$2 AND id=$3").bind(c).bind(id(d)?).bind(id(p)?).fetch_optional(&mut **tx).await.map_err(err)?.ok_or(Error::NotFound)?;
    if payment.get::<bool, _>("reversed") {
        return Err(Error::Conflict);
    }
    let sales: faktura_application::sales::Data =
        serde_json::from_value(r.get("data")).map_err(err)?;
    let mut data = sales.invoice;
    if data.calculation.tax_mode != TaxMode::VatPayer || data.supplier.vat_number.trim().is_empty()
    {
        return Err(Error::Invalid(vec!["tax_mode".into()]));
    }
    let mut totals: faktura_domain::invoice::Totals =
        serde_json::from_value(r.get("advance_totals")).map_err(err)?;
    let amount: String = payment.get("amount");
    let groups = faktura_domain::tax_receipt::allocate(&totals, &amount).map_err(Error::Invalid)?;
    data.tax_on = Some(payment.get("paid_on"));
    data.issued_on = data.tax_on;
    data.due_on = None;
    data.payment.method = payment.get("method");
    data.calculation.price_mode = PriceMode::Gross;
    data.calculation.rounding = Rounding::None;
    data.calculation.discount_percent = "0".into();
    data.calculation.lines = groups
        .iter()
        .map(|g| Line {
            catalog_id: None,
            id: format!("vat-{}", g.vat_percent),
            description: if data.language == "en" {
                "Received advance payment"
            } else {
                "Přijatá záloha"
            }
            .into(),
            unit: "".into(),
            quantity: "1".into(),
            unit_price: g.amounts.gross.clone(),
            discount_percent: "0".into(),
            vat_percent: g.vat_percent.clone(),
        })
        .collect();
    data.note = String::new();
    totals.calculation_version = faktura_domain::tax_receipt::VERSION.into();
    totals.lines = groups
        .iter()
        .map(|g| LineResult {
            id: format!("vat-{}", g.vat_percent),
            amounts: g.amounts.clone(),
        })
        .collect();
    let sum = |f: fn(&Amounts) -> &String| -> Result<String> {
        let mut n = Decimal::ZERO;
        for g in &groups {
            n += Decimal::from_str(f(&g.amounts)).map_err(err)?;
        }
        n.rescale(2);
        Ok(n.to_string())
    };
    totals.amounts = Amounts {
        net: sum(|a| &a.net)?,
        vat: sum(|a| &a.vat)?,
        gross: amount.clone(),
    };
    totals.payable = amount;
    totals.rounding = "0.00".into();
    totals.tax_summary = groups;
    Ok((
        Document {
            id: String::new(),
            number: String::new(),
            company_id: c.to_string(),
            advance_id: d.into(),
            advance_number: r.get("number"),
            payment_id: p.into(),
            data,
            totals,
        },
        r.get("revision"),
    ))
}
#[async_trait]
impl TaxReceiptStore for PgTaxReceipt {
    async fn preview(&self, a: &str, c: &str, d: &str, p: &str) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tax_receipt WHERE company_id=$1 AND advance_id=$2 AND payment_id=$3)").bind(c).bind(id(d)?).bind(id(p)?).fetch_one(&mut *tx).await.map_err(err)?;
        if exists {
            return Err(Error::Conflict);
        }
        Ok(source(&mut tx, c, d, p, false).await?.0)
    }
    async fn issue(&self, a: &str, c: &str, cmd: Issue) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let command = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM tax_receipt WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != a
                || r.get::<serde_json::Value, _>("command") != command
            {
                return Err(Error::Conflict);
            }
            return map(r);
        }
        let (mut doc, revision) =
            source(&mut tx, c, &cmd.advance_id, &cmd.payment_id, true).await?;
        crate::advance_settlement::ensure_free(&mut tx, c, id(&cmd.advance_id)?).await?;
        if revision != cmd.revision {
            return Err(Error::Conflict);
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tax_receipt WHERE company_id=$1 AND payment_id=$2)",
        )
        .bind(c)
        .bind(id(&cmd.payment_id)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if exists {
            return Err(Error::Conflict);
        }
        let total:bool=sqlx::query_scalar("SELECT COALESCE((SELECT SUM((snapshot->'totals'->>'payable')::numeric) FROM tax_receipt WHERE company_id=$1 AND advance_id=$2),0)+$3::text::numeric <= (SELECT (advance_totals->>'payable')::numeric FROM sales_document WHERE company_id=$1 AND id=$2)").bind(c).bind(id(&cmd.advance_id)?).bind(&doc.totals.payable).fetch_one(&mut *tx).await.map_err(err)?;
        if !total {
            return Err(Error::Invalid(vec!["amount".into()]));
        }
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        if Some(cmd.issued_on) < doc.data.tax_on || cmd.issued_on > today {
            return Err(Error::Invalid(vec!["issued_on".into()]));
        }
        let year = cmd.issued_on.format("%Y").to_string();
        let n:i64=sqlx::query_scalar("INSERT INTO tax_receipt_sequence(company_id,year,next_value) VALUES($1,$2::text::integer,2) ON CONFLICT(company_id,year) DO UPDATE SET next_value=tax_receipt_sequence.next_value+1 RETURNING next_value-1").bind(c).bind(&year).fetch_one(&mut *tx).await.map_err(err)?;
        doc.number = format!("DDPP{year}{n:04}");
        doc.data.issued_on = Some(cmd.issued_on);
        doc.data.language = cmd.language;
        for line in &mut doc.data.calculation.lines {
            line.description = cmd.description.clone();
        }
        let ident: i64 =
            sqlx::query_scalar("SELECT nextval(pg_get_serial_sequence('tax_receipt','id'))")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        doc.id = ident.to_string();
        let document_style: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT data FROM company_document_style WHERE company_id=$1")
                .bind(c)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        let style: Option<faktura_application::document_style::Style> = document_style
            .clone()
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?;
        let for_pdf = doc.clone();
        let pdf = tokio::task::spawn_blocking(move || {
            if let Some(style) = style.as_ref() {
                crate::invoice_pdf::render_tax_receipt_with_style(&for_pdf, Some(style))
            } else {
                crate::invoice_pdf::render_tax_receipt(&for_pdf)
            }
        })
        .await
        .map_err(err)?
        .map_err(err)?;
        sqlx::query("INSERT INTO tax_receipt(id,company_id,advance_id,payment_id,actor_id,request_id,command,number,snapshot,pdf,document_style) OVERRIDING SYSTEM VALUE VALUES($1,$2,$3,$4,$5,$6::text::uuid,$7,$8,$9,$10,$11)").bind(ident).bind(c).bind(id(&cmd.advance_id)?).bind(id(&cmd.payment_id)?).bind(a).bind(&cmd.request_id).bind(command).bind(&doc.number).bind(serde_json::to_value(&doc).map_err(err)?).bind(pdf).bind(document_style).execute(&mut *tx).await.map_err(err)?;
        let row=sqlx::query("UPDATE sales_document SET revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(&cmd.advance_id)?).fetch_one(&mut *tx).await.map_err(err)?;
        crate::sales::audit(&mut tx, a, &crate::sales::map(row)?, "tax_receipt_issue").await?;
        tx.commit().await.map_err(err)?;
        Ok(doc)
    }
    async fn get(&self, a: &str, c: &str, id_: &str) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        map(
            sqlx::query("SELECT snapshot FROM tax_receipt WHERE company_id=$1 AND id=$2")
                .bind(c)
                .bind(id(id_)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
        )
    }
    async fn pdf(&self, a: &str, c: &str, id_: &str) -> Result<Vec<u8>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        sqlx::query_scalar("SELECT pdf FROM tax_receipt WHERE company_id=$1 AND id=$2")
            .bind(c)
            .bind(id(id_)?)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)
    }
    async fn list(&self, a: &str, c: &str, s: Search) -> Result<Page> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let page = s.page.unwrap_or(1);
        let size = s.page_size.unwrap_or(25);
        let q = s.query.trim();
        let filter="company_id=$1 AND ($2='' OR strpos(lower(number||' '||(snapshot->>'advance_number')||' '||(snapshot->'data'->'customer'->>'name')),lower($2))>0)";
        let total = sqlx::query_scalar(&format!("SELECT count(*) FROM tax_receipt WHERE {filter}"))
            .bind(c)
            .bind(q)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        let items = sqlx::query(&format!(
            "SELECT snapshot FROM tax_receipt WHERE {filter} ORDER BY id DESC LIMIT $3 OFFSET $4"
        ))
        .bind(c)
        .bind(q)
        .bind(size)
        .bind((page - 1) * size)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?
        .into_iter()
        .map(map)
        .collect::<Result<Vec<_>>>()?;
        Ok(Page {
            items,
            total,
            page,
            page_size: size,
        })
    }
}

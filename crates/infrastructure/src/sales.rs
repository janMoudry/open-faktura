use crate::draft::{authorize, check_contact};
use async_trait::async_trait;
use faktura_application::sales::*;
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgSales(pub PgPool);
fn err(error: impl std::fmt::Display) -> Error {
    // PostgreSQL trigger protects content after an explicit document export lock.
    if error.to_string().contains("document_export_locked") {
        Error::Conflict
    } else {
        Error::Unavailable
    }
}
fn id(s: &str) -> Result<i64> {
    s.parse().map_err(|_| Error::NotFound)
}
fn kind(k: &Kind) -> &'static str {
    match k {
        Kind::Quote => "quote",
        Kind::Template => "template",
        Kind::Advance => "advance",
    }
}
pub(crate) fn map(r: sqlx::postgres::PgRow) -> Result<Document> {
    let mut data: Data = serde_json::from_value(r.get("data")).map_err(err)?;
    if r.try_get::<Option<String>, _>("linked_number")
        .ok()
        .flatten()
        .is_some()
    {
        data.status = "invoiced".into();
    }
    Ok(Document {
        document_style: r
            .get::<Option<serde_json::Value>, _>("document_style")
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?,
        id: r.get::<i64, _>("id").to_string(),
        company_id: r.get::<i64, _>("company_id").to_string(),
        kind: match r.get::<String, _>("kind").as_str() {
            "quote" => Kind::Quote,
            "advance" => Kind::Advance,
            _ => Kind::Template,
        },
        advance_paid: if r.get::<String, _>("kind") == "advance" {
            Some(
                r.get::<rust_decimal::Decimal, _>("advance_paid")
                    .to_string(),
            )
        } else {
            None
        },
        revision: r.get("revision"),
        archived: r.get("archived"),
        number: r.get("number"),
        invoice_id: r.get::<Option<i64>, _>("invoice_id").map(|v| v.to_string()),
        totals: if r.get::<String, _>("kind") == "advance" {
            r.try_get::<Option<serde_json::Value>, _>("advance_totals")
                .ok()
                .flatten()
                .map(serde_json::from_value)
                .transpose()
                .map_err(err)?
        } else {
            faktura_domain::invoice::calculate(&data.invoice.calculation).ok()
        },
        data,
    })
}
pub(crate) async fn ensure_not_trashed(
    tx: &mut Transaction<'_, Postgres>,
    company: i64,
    document: i64,
) -> Result<()> {
    let deleted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sales_trash WHERE company_id=$1 AND document_id=$2 AND deleted_at IS NOT NULL)").bind(company).bind(document).fetch_one(&mut **tx).await.map_err(err)?;
    if deleted {
        Err(Error::NotFound)
    } else {
        Ok(())
    }
}
pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    v: &Document,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO sales_audit(company_id,document_id,revision,actor_id,action,snapshot) VALUES($1,$2,$3,$4,$5,$6)").bind(id(&v.company_id)?).bind(id(&v.id)?).bind(v.revision).bind(actor).bind(action).bind(serde_json::to_value(v).map_err(err)?).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
const SELECT:&str="SELECT s.*,i.number AS linked_number FROM sales_document s LEFT JOIN invoice_draft i ON i.company_id=s.company_id AND i.id=s.invoice_id";
#[async_trait]
impl SalesStore for PgSales {
    async fn list(&self, a: &str, c: &str, s: Search) -> Result<Page> {
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        if s.tag_id.len() > 40
            || (!s.tag_id.is_empty() && s.tag_id.parse::<i64>().ok().is_none_or(|v| v <= 0))
        {
            return Err(Error::Invalid(vec!["tag_id".into()]));
        }
        let predicate="NOT EXISTS(SELECT 1 FROM sales_trash t WHERE t.company_id=s.company_id AND t.document_id=s.id AND t.deleted_at IS NOT NULL) AND s.company_id=$1 AND s.kind=$2 AND s.archived=$3 AND strpos(lower(concat_ws(' ',s.number,s.data->>'name',s.data->'invoice'->'customer'->>'name')),lower($4))>0 AND ($5='all' OR ($2='advance' AND (($5='paid' AND s.advance_paid >= (s.advance_totals->>'payable')::numeric) OR ($5='outstanding' AND s.advance_paid < (s.advance_totals->>'payable')::numeric) OR ($5='overdue' AND s.advance_paid < (s.advance_totals->>'payable')::numeric AND (s.data#>>'{invoice,due_on}')::date < (now() AT TIME ZONE 'Europe/Prague')::date))) OR CASE WHEN i.number IS NOT NULL THEN 'invoiced' ELSE s.data->>'status' END=$5) AND ($6='' OR EXISTS(SELECT 1 FROM document_tag_link l WHERE l.company_id=s.company_id AND l.sales_id=s.id AND l.tag_id::text=$6))";
        let total:i64=sqlx::query_scalar(&format!("SELECT count(*) FROM sales_document s LEFT JOIN invoice_draft i ON i.company_id=s.company_id AND i.id=s.invoice_id WHERE {predicate}" )).bind(c).bind(kind(&s.kind)).bind(s.archived).bind(&s.query).bind(&s.status).bind(&s.tag_id).fetch_one(&mut *tx).await.map_err(err)?;
        let items = sqlx::query(&format!(
            "{SELECT} WHERE {predicate} ORDER BY s.updated_at DESC,s.id DESC LIMIT $7 OFFSET $8"
        ))
        .bind(c)
        .bind(kind(&s.kind))
        .bind(s.archived)
        .bind(&s.query)
        .bind(&s.status)
        .bind(&s.tag_id)
        .bind(s.page_size)
        .bind((s.page - 1) * s.page_size)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?
        .into_iter()
        .map(map)
        .collect::<Result<Vec<_>>>()?;
        tx.commit().await.map_err(err)?;
        Ok(Page {
            items,
            total,
            page: s.page,
            page_size: s.page_size,
        })
    }
    async fn get(&self, a: &str, c: &str, d: &str) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let v = map(
            sqlx::query(&format!("{SELECT} WHERE s.company_id=$1 AND s.id=$2 AND NOT EXISTS(SELECT 1 FROM sales_trash t WHERE t.company_id=s.company_id AND t.document_id=s.id AND t.deleted_at IS NOT NULL)"))
                .bind(c)
                .bind(id(d)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
        )?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        // Serialize creation in the profile without upgrading the membership locks.
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let data = serde_json::to_value(&cmd.data).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM sales_document WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<serde_json::Value, _>("initial_data") != data
                || r.get::<i64, _>("creator_id") != a
                || r.get::<String, _>("kind") != kind(&cmd.kind)
            {
                return Err(Error::Conflict);
            }
            ensure_not_trashed(&mut tx, c, r.get("id")).await?;
            return map(r);
        }
        check_contact(&mut tx, c, cmd.data.invoice.contact_id.as_deref()).await?;
        crate::stock_documents::validate_links(&mut tx, c, &cmd.data.invoice.calculation).await?;
        let number = if cmd.kind != Kind::Template {
            let year = cmd
                .data
                .invoice
                .issued_on
                .ok_or_else(|| Error::Invalid(vec!["issued_on".into()]))?
                .format("%Y")
                .to_string();
            let table = if cmd.kind == Kind::Advance {
                "advance_sequence"
            } else {
                "quote_sequence"
            };
            let n:i64=sqlx::query_scalar(&format!("INSERT INTO {table}(company_id,year,next_value) VALUES($1,$2::text::integer,2) ON CONFLICT(company_id,year) DO UPDATE SET next_value={table}.next_value+1 RETURNING next_value-1")).bind(c).bind(&year).fetch_one(&mut *tx).await.map_err(err)?;
            Some(format!(
                "{}{year}{n:04}",
                if cmd.kind == Kind::Advance {
                    "ZF"
                } else {
                    "CN"
                }
            ))
        } else {
            None
        };
        let mut v=map(sqlx::query("INSERT INTO sales_document(company_id,kind,creator_id,request_id,initial_data,data,number,document_style) VALUES($1,$2,$3,$4::text::uuid,$5,$5,$6,(SELECT data FROM company_document_style WHERE company_id=$1)) RETURNING *").bind(c).bind(kind(&cmd.kind)).bind(a).bind(&cmd.request_id).bind(data).bind(number).fetch_one(&mut *tx).await.map_err(err)?)?;
        if v.kind == Kind::Advance {
            persist_advance(&mut tx, &mut v).await?;
        }
        audit(&mut tx, a, &v, "create").await?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
    async fn update(&self, a: &str, c: &str, d: &str, cmd: Update) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        let old = map(sqlx::query(&format!(
            "{SELECT} WHERE s.company_id=$1 AND s.id=$2 FOR UPDATE OF s"
        ))
        .bind(c)
        .bind(id(d)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?)?;
        ensure_not_trashed(&mut tx, c, id(d)?).await?;
        if old.revision != cmd.revision
            || (old.invoice_id.is_some()
                && serde_json::to_value(&old.data).map_err(err)?
                    != serde_json::to_value(&cmd.data).map_err(err)?)
        {
            return Err(Error::Conflict);
        }
        if old.kind == Kind::Advance
            && old
                .advance_paid
                .as_deref()
                .unwrap_or("0")
                .parse::<rust_decimal::Decimal>()
                .map_err(err)?
                > rust_decimal::Decimal::ZERO
        {
            return Err(Error::Conflict);
        }
        crate::draft::check_existing_contact(
            &mut tx,
            c,
            cmd.data.invoice.contact_id.as_deref(),
            old.data.invoice.contact_id.as_deref(),
        )
        .await?;
        crate::stock_documents::validate_links(&mut tx, c, &cmd.data.invoice.calculation).await?;
        let mut v=map(sqlx::query("UPDATE sales_document SET document_style=CASE WHEN data IS DISTINCT FROM $3 THEN (SELECT data FROM company_document_style WHERE company_id=$1) ELSE document_style END,data=$3,archived=$4,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(d)?).bind(serde_json::to_value(cmd.data).map_err(err)?).bind(cmd.archived).fetch_one(&mut *tx).await.map_err(err)?)?;
        if v.kind == Kind::Advance {
            persist_advance(&mut tx, &mut v).await?;
        }
        audit(&mut tx, a, &v, "update").await?;
        tx.commit().await.map_err(err)?;
        Ok(v)
    }
    async fn convert(
        &self,
        a: &str,
        c: &str,
        d: &str,
        cmd: Convert,
    ) -> Result<faktura_application::draft::Draft> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let mut old = map(sqlx::query(
            "SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(c)
        .bind(id(d)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?)?;
        ensure_not_trashed(&mut tx, c, id(d)?).await?;
        if old.kind == Kind::Advance {
            return Err(Error::Invalid(vec![
                "advance_settlement_not_available".into()
            ]));
        }
        let command = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM sales_conversion WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != a
                || r.get::<i64, _>("document_id") != id(d)?
                || r.get::<serde_json::Value, _>("command") != command
            {
                return Err(Error::Conflict);
            }
            return crate::draft::map(
                sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2")
                    .bind(c)
                    .bind(r.get::<i64, _>("invoice_id"))
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(err)?,
            );
        }
        if old.kind == Kind::Quote {
            if let Some(invoice) = old.invoice_id {
                return crate::draft::map(
                    sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2")
                        .bind(c)
                        .bind(id(&invoice)?)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(err)?,
                );
            }
        }
        if old.archived || old.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        let mut data = old.data.invoice.clone();
        if old.kind == Kind::Quote {
            data.note = old.data.name.clone();
        }
        data.issued_on = Some(cmd.issued_on);
        data.due_on = cmd
            .issued_on
            .checked_add_signed(chrono::Duration::days(old.data.maturity_days));
        data.tax_on = if data.calculation.tax_mode == faktura_domain::invoice::TaxMode::VatPayer {
            Some(cmd.issued_on)
        } else {
            None
        };
        if data
            .due_on
            .is_none_or(|d| !("1900-01-01"..="9999-12-31").contains(&d.to_string().as_str()))
        {
            return Err(Error::Invalid(vec!["due_on".into()]));
        }
        data.payment.variable_symbol.clear();
        let errors = faktura_domain::invoice::validate_draft(&data);
        if !errors.is_empty() {
            return Err(Error::Invalid(errors));
        }
        check_contact(&mut tx, c, data.contact_id.as_deref()).await?;
        crate::stock_documents::validate_links(&mut tx, c, &data.calculation).await?;
        let totals = faktura_domain::invoice::calculate(&data.calculation).ok();
        let json = serde_json::to_value(&data).map_err(err)?;
        let row=sqlx::query("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,contact_id) VALUES($1,$2,$3::text::uuid,$4,$4,$5,$6) RETURNING *").bind(c).bind(a).bind(uuid::Uuid::new_v4().to_string()).bind(json).bind(serde_json::to_value(totals).map_err(err)?).bind(data.contact_id.as_deref().map(id).transpose()?).fetch_one(&mut *tx).await.map_err(err)?;
        let invoice = crate::draft::map(row)?;
        crate::draft::audit(&mut tx, a, &invoice, "create").await?;
        sqlx::query("INSERT INTO sales_conversion(company_id,request_id,document_id,actor_id,command,invoice_id) VALUES($1,$2::text::uuid,$3,$4,$5,$6)").bind(c).bind(&cmd.request_id).bind(id(d)?).bind(a).bind(command).bind(id(&invoice.id)?).execute(&mut *tx).await.map_err(err)?;
        if old.kind == Kind::Quote {
            old.invoice_id = Some(invoice.id.clone());
            old.revision += 1;
            sqlx::query("UPDATE sales_document SET invoice_id=$3,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(c).bind(id(d)?).bind(id(&invoice.id)?).execute(&mut *tx).await.map_err(err)?;
            audit(&mut tx, a, &old, "convert").await?;
        }
        tx.commit().await.map_err(err)?;
        Ok(invoice)
    }
}

async fn persist_advance(tx: &mut Transaction<'_, Postgres>, v: &mut Document) -> Result<()> {
    v.totals = Some(
        faktura_domain::invoice::calculate(&v.data.invoice.calculation).map_err(Error::Invalid)?,
    );
    if v.data.invoice.payment.variable_symbol.is_empty() {
        v.data.invoice.payment.variable_symbol = v
            .number
            .as_deref()
            .unwrap_or("")
            .trim_start_matches("ZF")
            .into();
    }
    let doc = faktura_application::draft::Draft {
        advances: None,
        id: v.id.clone(),
        company_id: v.company_id.clone(),
        revision: v.revision,
        data: v.data.invoice.clone(),
        totals: v.totals.clone(),
        number: v.number.clone(),
        settlement: None,
        issued_at: None,
    };
    let style = v.document_style.clone();
    let pdf = tokio::task::spawn_blocking(move || {
        if let Some(style) = style.as_ref() {
            crate::invoice_pdf::render_advance_with_style(&doc, Some(style))
        } else {
            crate::invoice_pdf::render_advance(&doc)
        }
    })
    .await
    .map_err(err)?
    .map_err(err)?;
    sqlx::query("UPDATE sales_document SET data=$3,advance_totals=$4,advance_pdf=$5 WHERE company_id=$1 AND id=$2").bind(id(&v.company_id)?).bind(id(&v.id)?).bind(serde_json::to_value(&v.data).map_err(err)?).bind(serde_json::to_value(&v.totals).map_err(err)?).bind(pdf).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}

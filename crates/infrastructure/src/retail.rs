#[path = "retail_posting.rs"]
mod posting;
use async_trait::async_trait;
use chrono::Datelike;
use faktura_application::{
    company::{CompanyError as Error, Result},
    retail::*,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgRetail(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn invalid(v: &str) -> Error {
    Error::Invalid(vec![v.into()])
}
fn map(r: sqlx::postgres::PgRow) -> Result<Document> {
    Ok(Document {
        posting_id: r
            .try_get::<Option<i64>, _>("posting_id")
            .unwrap_or(None)
            .map(|v| v.to_string()),
        id: r.get::<i64, _>("id").to_string(),
        number: r.get("number"),
        revision: r.get("revision"),
        data: serde_json::from_value(r.get("data")).map_err(err)?,
        totals: serde_json::from_value(r.get("totals")).map_err(err)?,
        allocations: serde_json::from_value(r.get("allocations")).map_err(err)?,
        terminal: r
            .get::<Option<serde_json::Value>, _>("terminal")
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?,
        cancelled: r.get("cancelled"),
    })
}
async fn load(tx: &mut Transaction<'_, Postgres>, c: i64, i: &str) -> Result<Document> {
    map(
        sqlx::query("SELECT * FROM retail_document WHERE company_id=$1 AND id=$2 FOR UPDATE")
            .bind(c)
            .bind(id(i)?)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?,
    )
}
async fn prepare(tx: &mut Transaction<'_, Postgres>, c: i64, d: &mut Data) -> Result<Preview> {
    if let Some(contact) = &d.invoice.contact_id {
        if !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM contact WHERE company_id=$1 AND id=$2)",
        )
        .bind(c)
        .bind(id(contact)?)
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?
        {
            return Err(Error::NotFound);
        }
    }
    let settings: faktura_domain::company::CompanySettings = serde_json::from_value(
        sqlx::query_scalar::<_, serde_json::Value>("SELECT settings FROM company WHERE id=$1")
            .bind(c)
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?,
    )
    .map_err(err)?;
    if settings.tax_mode
        != match d.invoice.calculation.tax_mode {
            faktura_domain::invoice::TaxMode::VatPayer => "vat_payer",
            _ => "non_payer",
        }
    {
        return Err(invalid("tax_mode"));
    }
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    if d.invoice.issued_on.is_none_or(|v| v > today) {
        return Err(invalid("issued_on"));
    }
    if [
        &settings.name,
        &settings.registration_number,
        &settings.street,
        &settings.city,
        &settings.postal_code,
    ]
    .iter()
    .any(|v| v.trim().is_empty())
    {
        return Err(invalid("retail_profile"));
    }
    if settings.tax_mode == "vat_payer" {
        if settings.vat_number.trim().is_empty() {
            return Err(invalid("retail_profile"));
        }
        if !settings.tax_effective_from.is_empty() {
            let from: chrono::NaiveDate = settings.tax_effective_from.parse().map_err(err)?;
            if d.invoice.issued_on.is_some_and(|v| v < from) {
                return Err(invalid("issued_on"));
            }
        }
    }
    d.invoice.supplier = faktura_domain::invoice::PartySnapshot {
        name: settings.name,
        registration_number: settings.registration_number,
        vat_number: settings.vat_number,
        email: settings.email,
        phone: settings.phone,
        address: faktura_domain::contact::Address {
            street: settings.street,
            city: settings.city,
            postal_code: settings.postal_code,
            country: settings.country,
        },
    };
    d.invoice.due_on = d.invoice.issued_on;
    d.invoice.tax_on = if settings.tax_mode == "vat_payer" {
        d.invoice.issued_on
    } else {
        None
    };
    faktura_domain::retail::preview(d).map_err(Error::Invalid)
}
#[async_trait]
impl RetailStore for PgRetail {
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        if q.query.len() > 200 || !(1..=10000).contains(&q.page.unwrap_or(1)) {
            return Err(invalid("search"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let predicate="company_id=$1 AND cancelled=$2 AND ($3='' OR strpos(lower(number||' '||coalesce(data->'invoice'->'customer'->>'name','')||' '||coalesce(terminal->>'name','')),lower($3))>0)";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM retail_document WHERE {predicate}"
        ))
        .bind(c)
        .bind(q.cancelled)
        .bind(&q.query)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let rows = sqlx::query(&format!(
            "SELECT * FROM retail_document WHERE {predicate} ORDER BY id DESC LIMIT 25 OFFSET $4"
        ))
        .bind(c)
        .bind(q.cancelled)
        .bind(q.query)
        .bind((q.page.unwrap_or(1) - 1) * 25)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
        Ok(Page {
            items: rows.into_iter().map(map).collect::<Result<_>>()?,
            total,
            page: q.page.unwrap_or(1),
        })
    }
    async fn get(&self, a: &str, c: &str, i: &str) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c, i).await
    }
    async fn preview(&self, a: &str, c: &str, mut data: Data) -> Result<Preview> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        prepare(&mut tx, c, &mut data).await
    }
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Document> {
        if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
            return Err(invalid("request_id"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM retail_command WHERE company_id=$1 AND request_id=$2::text::uuid",
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
        let reason = match &cmd.action {
            Action::Save { reason, .. } | Action::Cancel { reason, .. } => reason.clone(),
        };
        if reason.trim().is_empty()
            || reason.chars().count() > 500
            || reason.chars().any(char::is_control)
        {
            return Err(invalid("reason"));
        }
        let result = match cmd.action {
            Action::Save {
                id: existing,
                revision,
                mut data,
                ..
            } => {
                let preview = prepare(&mut tx, c, &mut data).await?;
                let terminal = if let Some(i) = &data.terminal_id {
                    let r=sqlx::query("SELECT * FROM sales_terminal WHERE company_id=$1 AND id=$2 AND NOT archived").bind(c).bind(id(i)?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::Conflict)?;
                    Some(faktura_application::cash::Terminal {
                        id: r.get::<i64, _>("id").to_string(),
                        revision: r.get("revision"),
                        code: r.get("code"),
                        name: r.get("name"),
                        archived: false,
                    })
                } else {
                    None
                };
                let document_id = if let Some(i) = existing {
                    let old = load(&mut tx, c, &i).await?;
                    if Some(old.revision) != revision || old.cancelled || old.posting_id.is_some() {
                        return Err(Error::Conflict);
                    }
                    sqlx::query("UPDATE retail_document SET revision=revision+1,pdf=NULL,updated_at=now() WHERE company_id=$1 AND id=$2").bind(c).bind(id(&i)?).execute(&mut *tx).await.map_err(err)?;
                    i
                } else {
                    if revision.is_some() {
                        return Err(invalid("revision"));
                    }
                    let year = data.invoice.issued_on.unwrap().year();
                    let seq:i64=sqlx::query_scalar("INSERT INTO retail_series(company_id,year,next_value) VALUES($1,$2,2) ON CONFLICT(company_id,year) DO UPDATE SET next_value=retail_series.next_value+1 RETURNING next_value-1").bind(c).bind(year).fetch_one(&mut *tx).await.map_err(err)?;
                    sqlx::query_scalar::<_,i64>("INSERT INTO retail_document(company_id,number,data,totals,allocations) VALUES($1,$2,'{}','{}','[]') RETURNING id").bind(c).bind(format!("PR{year}{seq:04}")).fetch_one(&mut *tx).await.map_err(err)?.to_string()
                };
                sqlx::query("UPDATE retail_document SET data=$3,totals=$4,allocations=$5,terminal_id=$6,terminal=$7 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&document_id)?).bind(serde_json::to_value(&data).map_err(err)?).bind(serde_json::to_value(&preview.totals).map_err(err)?).bind(serde_json::to_value(&preview.allocations).map_err(err)?).bind(data.terminal_id.as_deref().map(id).transpose()?).bind(terminal.map(serde_json::to_value).transpose().map_err(err)?).execute(&mut *tx).await.map_err(err)?;
                load(&mut tx, c, &document_id).await?
            }
            Action::Cancel {
                id: i, revision, ..
            } => {
                let old = load(&mut tx, c, &i).await?;
                if old.cancelled || old.revision != revision || old.posting_id.is_some() {
                    return Err(Error::Conflict);
                }
                sqlx::query("UPDATE retail_document SET cancelled=true,revision=revision+1,updated_at=now(),pdf=NULL WHERE company_id=$1 AND id=$2").bind(c).bind(id(&i)?).execute(&mut *tx).await.map_err(err)?;
                load(&mut tx, c, &i).await?
            }
        };
        crate::stock_documents::retail(
            &mut tx,
            a,
            c,
            id(&result.id)?,
            &result.data.invoice,
            result.cancelled,
        )
        .await?;
        let snapshot = serde_json::to_value(&result).map_err(err)?;
        sqlx::query("INSERT INTO retail_audit(company_id,document_id,actor_id,revision,reason,snapshot) VALUES($1,$2,$3,$4,$5,$6)").bind(c).bind(id(&result.id)?).bind(a).bind(result.revision).bind(reason).bind(&snapshot).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("INSERT INTO retail_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(cmd.request_id).bind(a).bind(input).bind(snapshot).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn pdf(&self, a: &str, c: &str, i: &str) -> Result<Vec<u8>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let d = load(&mut tx, c, i).await?;
        if let Some(bytes) = sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT pdf FROM retail_document WHERE company_id=$1 AND id=$2",
        )
        .bind(c)
        .bind(id(i)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?
        {
            return Ok(bytes);
        }
        let bytes = tokio::task::spawn_blocking(move || crate::retail_pdf::render(&d))
            .await
            .map_err(err)?
            .map_err(err)?;
        sqlx::query("UPDATE retail_document SET pdf=$3 WHERE company_id=$1 AND id=$2")
            .bind(c)
            .bind(id(i)?)
            .bind(&bytes)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(bytes)
    }
    async fn history(&self, a: &str, c: &str, i: &str) -> Result<Vec<Audit>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c, i).await?;
        Ok(sqlx::query(
            "SELECT * FROM retail_audit WHERE company_id=$1 AND document_id=$2 ORDER BY id DESC",
        )
        .bind(c)
        .bind(id(i)?)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?
        .into_iter()
        .map(|r| Audit {
            revision: r.get("revision"),
            reason: r.get("reason"),
            actor_id: r.get::<i64, _>("actor_id").to_string(),
            created_at: r
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
        })
        .collect())
    }
}

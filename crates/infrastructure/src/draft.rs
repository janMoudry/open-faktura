use async_trait::async_trait;
use faktura_application::draft::{
    Create, Draft, DraftPage, DraftStore, Error, Issue, Result, Search, Update,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgDraft(pub PgPool);
fn unavailable(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(value: &str) -> Result<i64> {
    value.parse().map_err(|_| Error::NotFound)
}
pub(crate) fn map(row: sqlx::postgres::PgRow) -> Result<Draft> {
    Ok(Draft {
        id: row.get::<i64, _>("id").to_string(),
        company_id: row.get::<i64, _>("company_id").to_string(),
        revision: row.get("revision"),
        settlement: row
            .try_get::<Option<serde_json::Value>, _>("settlement")
            .ok()
            .flatten()
            .map(serde_json::from_value)
            .transpose()
            .map_err(unavailable)?,
        number: row.get("number"),
        issued_at: row
            .get::<Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>, _>("issued_at")
            .map(|v| v.to_rfc3339()),
        totals: row
            .get::<Option<serde_json::Value>, _>("totals")
            .filter(|v| !v.is_null())
            .map(serde_json::from_value)
            .transpose()
            .map_err(unavailable)?,
        data: serde_json::from_value(row.get("data")).map_err(unavailable)?,
    })
}
pub(crate) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    write: bool,
) -> Result<(i64, i64)> {
    let actor = id(actor)?;
    let company = id(company)?;
    let role: String = sqlx::query_scalar("SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR SHARE OF c,m")
        .bind(company).bind(actor).fetch_optional(&mut **tx).await.map_err(unavailable)?.ok_or(Error::NotFound)?;
    if write && !["owner", "editor"].contains(&role.as_str()) {
        return Err(Error::Forbidden);
    }
    Ok((actor, company))
}
pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    value: &Draft,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO invoice_draft_audit(company_id,draft_id,revision,actor_id,action,data,totals) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(id(&value.company_id)?).bind(id(&value.id)?).bind(value.revision).bind(actor).bind(action).bind(serde_json::to_value(&value.data).map_err(unavailable)?).bind(serde_json::to_value(&value.totals).map_err(unavailable)?).execute(&mut **tx).await.map_err(unavailable)?;
    Ok(())
}
#[async_trait]
impl DraftStore for PgDraft {
    async fn issue(
        &self,
        actor: &str,
        company: &str,
        invoice: &str,
        command: Issue,
    ) -> Result<Draft> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let actor = id(actor)?;
        let company = id(company)?;
        // Acquire the profile lock first (never upgrade a shared lock). Serializes settings,
        // membership changes, numbering and all issue requests within this profile.
        let profile = sqlx::query("SELECT c.settings,c.revision,m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR UPDATE OF c,m")
            .bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(Error::NotFound)?;
        if !["owner", "editor"].contains(&profile.get::<String, _>("role").as_str()) {
            return Err(Error::Forbidden);
        }
        let invoice = id(invoice)?;
        if let Some(row) = sqlx::query(
            "SELECT * FROM invoice_draft WHERE company_id=$1 AND issue_request_id=$2::text::uuid",
        )
        .bind(company)
        .bind(&command.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(unavailable)?
        {
            if row.get::<i64, _>("id") != invoice
                || row.get::<i64, _>("issue_actor_id") != actor
                || row.get::<i64, _>("issue_revision") != command.revision
            {
                return Err(Error::Conflict);
            }
            return map(row);
        }
        let mut draft = map(sqlx::query(
            "SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(company)
        .bind(invoice)
        .fetch_optional(&mut *tx)
        .await
        .map_err(unavailable)?
        .ok_or(Error::NotFound)?)?;
        if draft.number.is_some() || draft.revision != command.revision {
            return Err(Error::Conflict);
        }
        let totals =
            faktura_domain::invoice::validate_issue(&draft.data).map_err(Error::Invalid)?;
        let mut settings: faktura_domain::company::CompanySettings =
            serde_json::from_value(profile.get("settings")).map_err(unavailable)?;
        let date = if draft.data.calculation.tax_mode == faktura_domain::invoice::TaxMode::VatPayer
        {
            draft.data.tax_on
        } else {
            draft.data.issued_on
        }
        .expect("validated date")
        .to_string();
        let mode: Option<String> = sqlx::query_scalar("SELECT mode FROM company_tax_profile WHERE company_id=$1 AND effective_from <= $2::text::date ORDER BY effective_from DESC,revision DESC LIMIT 1")
            .bind(company).bind(date).fetch_optional(&mut *tx).await.map_err(unavailable)?;
        let has_history: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM company_tax_profile WHERE company_id=$1)",
        )
        .bind(company)
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        if mode.is_none() && has_history {
            return Err(Error::Invalid(vec!["tax_mode".into()]));
        }
        let mode = mode.unwrap_or(settings.tax_mode.clone());
        let draft_mode =
            serde_json::to_value(draft.data.calculation.tax_mode).map_err(unavailable)?;
        if draft_mode.as_str() != Some(mode.as_str()) {
            return Err(Error::Invalid(vec!["tax_mode".into()]));
        }
        let year = draft
            .data
            .issued_on
            .expect("validated date")
            .format("%Y")
            .to_string();
        if year != settings.number_year.to_string() {
            return Err(Error::Invalid(vec!["number_year".into()]));
        }
        sqlx::query("INSERT INTO invoice_number_series(company_id,prefix,year,next_value) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(company).bind(&settings.number_prefix).bind(settings.number_year).bind(settings.next_number).execute(&mut *tx).await.map_err(unavailable)?;
        let next: i32 = sqlx::query_scalar("SELECT GREATEST(next_value,$4) FROM invoice_number_series WHERE company_id=$1 AND prefix=$2 AND year=$3 FOR UPDATE")
            .bind(company).bind(&settings.number_prefix).bind(settings.number_year).bind(settings.next_number).fetch_one(&mut *tx).await.map_err(unavailable)?;
        if next > 999999 {
            return Err(Error::Invalid(vec!["next_number".into()]));
        }
        settings.next_number = next;
        let number = settings.number_preview();
        if draft.data.payment.variable_symbol.is_empty() {
            draft.data.payment.variable_symbol = format!("{}{:06}", settings.number_year, next);
        }
        let value = map(sqlx::query("UPDATE invoice_draft SET number=$3,issued_at=now(),issue_request_id=$4::text::uuid,issue_actor_id=$5,issue_revision=$6,data=$7,totals=$8,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *")
            .bind(company).bind(invoice).bind(&number).bind(&command.request_id).bind(actor).bind(command.revision)
            .bind(serde_json::to_value(&draft.data).map_err(unavailable)?).bind(serde_json::to_value(totals).map_err(unavailable)?)
            .fetch_one(&mut *tx).await.map_err(|e| if e.as_database_error().is_some_and(|d| d.is_unique_violation()) { Error::Conflict } else { unavailable(e) })?)?;
        sqlx::query("UPDATE invoice_number_series SET next_value=$4 WHERE company_id=$1 AND prefix=$2 AND year=$3")
            .bind(company).bind(&settings.number_prefix).bind(settings.number_year).bind(next+1).execute(&mut *tx).await.map_err(unavailable)?;
        settings.next_number = next + 1;
        let settings = serde_json::to_value(settings).map_err(unavailable)?;
        sqlx::query(
            "UPDATE company SET settings=$2,revision=revision+1,updated_at=now() WHERE id=$1",
        )
        .bind(company)
        .bind(&settings)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        sqlx::query("INSERT INTO company_audit(company_id,revision,actor_id,action,settings) VALUES($1,$2,$3,'update',$4)")
            .bind(company).bind(profile.get::<i64,_>("revision")+1).bind(actor).bind(settings).execute(&mut *tx).await.map_err(unavailable)?;
        audit(&mut tx, actor, &value, "issue").await?;
        sqlx::query("UPDATE invoice_draft_audit SET number=$3 WHERE company_id=$1 AND draft_id=$2 AND revision=$4")
            .bind(company).bind(invoice).bind(number).bind(value.revision).execute(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(value)
    }
    async fn pdf(&self, actor: &str, company: &str, invoice: &str) -> Result<Vec<u8>> {
        let value = self.get(actor, company, invoice).await?;
        if value.number.is_none() || value.data.calculation.calculation_version == "import-v1" {
            return Err(Error::Conflict);
        }
        // Bounded CPU work, outside a transaction; reauthorize before returning/cache write.
        static RENDERERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
        let _permit = RENDERERS.acquire().await.map_err(unavailable)?;
        let existing: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT pdf FROM invoice_draft WHERE company_id=$1 AND id=$2")
                .bind(id(company)?)
                .bind(id(invoice)?)
                .fetch_one(&self.0)
                .await
                .map_err(unavailable)?;
        let bytes = if let Some(bytes) = existing {
            bytes
        } else {
            tokio::task::spawn_blocking(move || crate::invoice_pdf::render(&value))
                .await
                .map_err(unavailable)?
                .map_err(unavailable)?
        };
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (_, company) = authorize(&mut tx, actor, company, false).await?;
        let bytes = sqlx::query_scalar("UPDATE invoice_draft SET pdf=COALESCE(pdf,$3) WHERE company_id=$1 AND id=$2 RETURNING pdf")
            .bind(company).bind(id(invoice)?).bind(bytes).fetch_one(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(bytes)
    }

    async fn list(&self, actor: &str, company: &str, search: Search) -> Result<DraftPage> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (_, company) = authorize(&mut tx, actor, company, false).await?;
        let predicate = "company_id=$1 AND ($2::bigint IS NULL OR contact_id=$2)
            AND strpos(lower(concat_ws(' ',data#>>'{customer,name}',data->>'order_number',data#>>'{calculation,lines,0,description}',number,id::text)),lower($3))>0
            AND ($4 = 'all' OR ($4 = 'draft' AND number IS NULL) OR ($4 = 'issued' AND number IS NOT NULL) OR ($4 = 'outstanding' AND (settlement->>'outstanding')::numeric>0) OR ($4 = 'overdue' AND (settlement->>'overdue')::boolean) OR ($4 IN ('paid','partial','overpaid') AND settlement->>'status'=$4))
            AND ($6::text IS NULL OR data->>$5 >= $6)
            AND ($7::text IS NULL OR data->>$5 <= $7)
            AND ($8::text IS NULL OR (totals->>'payable')::numeric >= $8::numeric)
            AND ($9::text IS NULL OR (totals->>'payable')::numeric <= $9::numeric)";
        let sort = match search.sort.as_str() {
            "issued_desc" => "data->>'issued_on' DESC NULLS LAST,id DESC",
            "issued_asc" => "data->>'issued_on' ASC NULLS LAST,id",
            "due_desc" => "data->>'due_on' DESC NULLS LAST,id DESC",
            "due_asc" => "data->>'due_on' ASC NULLS LAST,id",
            "customer_asc" => "lower(data#>>'{customer,name}') ASC NULLS LAST,id",
            "customer_desc" => "lower(data#>>'{customer,name}') DESC NULLS LAST,id DESC",
            "number_asc" => "number ASC NULLS LAST,id",
            "number_desc" => "number DESC NULLS LAST,id DESC",
            "amount_asc" => "(totals->>'payable')::numeric ASC NULLS LAST,id",
            "amount_desc" => "(totals->>'payable')::numeric DESC NULLS LAST,id DESC",
            _ => "updated_at DESC,id DESC",
        };
        let customer = search.customer_id.as_deref().map(id).transpose()?;
        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM invoice_with_settlement WHERE {predicate}"
        ))
        .bind(company)
        .bind(customer)
        .bind(&search.query)
        .bind(&search.status)
        .bind(&search.date_field)
        .bind(&search.date_from)
        .bind(&search.date_to)
        .bind(&search.amount_min)
        .bind(&search.amount_max)
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
        let items = sqlx::query(&format!(
            "SELECT * FROM invoice_with_settlement WHERE {predicate} ORDER BY {sort} LIMIT $10 OFFSET $11"
        ))
            .bind(company).bind(customer).bind(&search.query).bind(&search.status)
            .bind(&search.date_field).bind(&search.date_from).bind(&search.date_to)
            .bind(&search.amount_min).bind(&search.amount_max)
            .bind(search.page_size).bind((search.page - 1) * search.page_size)
            .fetch_all(&mut *tx).await.map_err(unavailable)?
            .into_iter().map(map).collect::<Result<Vec<_>>>()?;
        tx.commit().await.map_err(unavailable)?;
        Ok(DraftPage {
            items,
            total,
            page: search.page,
            page_size: search.page_size,
        })
    }
    async fn get(&self, actor: &str, company: &str, contact: &str) -> Result<Draft> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (_, company) = authorize(&mut tx, actor, company, false).await?;
        let value = map(sqlx::query(
            "SELECT * FROM invoice_with_settlement WHERE company_id=$1 AND id=$2",
        )
        .bind(company)
        .bind(id(contact)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(unavailable)?
        .ok_or(Error::NotFound)?)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(value)
    }
    async fn create(&self, actor: &str, company: &str, command: Create) -> Result<Draft> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (actor, company) = authorize(&mut tx, actor, company, true).await?;
        check_contact(&mut tx, company, command.data.contact_id.as_deref()).await?;
        let totals = faktura_domain::invoice::calculate(&command.data.calculation).ok();
        let data = serde_json::to_value(&command.data).map_err(unavailable)?;
        let inserted = sqlx::query("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,contact_id) VALUES ($1,$2,$3::text::uuid,$4,$4,$5,$6) ON CONFLICT(company_id,request_id) DO NOTHING RETURNING *")
            .bind(company).bind(actor).bind(&command.request_id).bind(&data).bind(serde_json::to_value(totals).map_err(unavailable)?).bind(command.data.contact_id.as_deref().map(id).transpose()?).fetch_optional(&mut *tx).await.map_err(unavailable)?;
        let value = if let Some(row) = inserted {
            let value = map(row)?;
            audit(&mut tx, actor, &value, "create").await?;
            value
        } else {
            let row = sqlx::query(
                "SELECT * FROM invoice_draft WHERE company_id=$1 AND request_id=$2::text::uuid",
            )
            .bind(company)
            .bind(&command.request_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
            if row.get::<serde_json::Value, _>("initial_data") != data
                || row.get::<i64, _>("creator_id") != actor
            {
                return Err(Error::Conflict);
            }
            map(row)?
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(value)
    }
    async fn update(
        &self,
        actor: &str,
        company: &str,
        contact: &str,
        command: Update,
    ) -> Result<Draft> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (actor, company) = authorize(&mut tx, actor, company, true).await?;
        let contact = id(contact)?;
        let previous = map(sqlx::query(
            "SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(company)
        .bind(contact)
        .fetch_optional(&mut *tx)
        .await
        .map_err(unavailable)?
        .ok_or(Error::NotFound)?)?;
        if previous.number.is_some() || previous.revision != command.revision {
            return Err(Error::Conflict);
        }
        check_contact(&mut tx, company, command.data.contact_id.as_deref()).await?;
        let totals = faktura_domain::invoice::calculate(&command.data.calculation).ok();
        let value = map(sqlx::query("UPDATE invoice_draft SET data=$3,totals=$4,contact_id=$5,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *")
            .bind(company).bind(contact).bind(serde_json::to_value(&command.data).map_err(unavailable)?).bind(serde_json::to_value(totals).map_err(unavailable)?).bind(command.data.contact_id.as_deref().map(id).transpose()?).fetch_one(&mut *tx).await.map_err(unavailable)?)?;
        audit(&mut tx, actor, &value, "update").await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(value)
    }
}

pub(crate) async fn check_contact(
    tx: &mut Transaction<'_, Postgres>,
    company: i64,
    contact: Option<&str>,
) -> Result<()> {
    if let Some(contact) = contact {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM contact WHERE company_id=$1 AND id=$2)",
        )
        .bind(company)
        .bind(id(contact)?)
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)?;
        if !exists {
            return Err(Error::NotFound);
        }
    }
    Ok(())
}

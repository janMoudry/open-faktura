use async_trait::async_trait;
use faktura_application::company::{Company, CompanyError, CompanyStore, Result, TaxProfile};
use faktura_domain::company::CompanySettings;
use sqlx::{PgPool, Row};

#[derive(Clone)]
pub struct PgCompany(pub PgPool);
fn unavailable(_: impl std::fmt::Display) -> CompanyError {
    CompanyError::Unavailable
}
fn id(value: &str) -> Result<i64> {
    value.parse().map_err(|_| CompanyError::NotFound)
}
fn map(row: sqlx::postgres::PgRow) -> Result<Company> {
    let settings: CompanySettings =
        serde_json::from_value(row.get("settings")).map_err(unavailable)?;
    let tax_history: Vec<TaxProfile> =
        serde_json::from_value(row.get("tax_history")).map_err(unavailable)?;
    Ok(Company {
        id: row.get::<i64, _>("id").to_string(),
        revision: row.get("revision"),
        step: row.get("step"),
        completed: row.get("completed"),
        role: row.get("role"),
        missing_fields: settings.issues(true),
        number_preview: settings.number_preview(),
        can_issue: settings.issues(true).is_empty()
            && settings.next_number <= 999999
            && row.get::<String, _>("role") != "viewer",
        settings,
        tax_history,
    })
}
const SELECT: &str = "SELECT c.id,c.settings,c.revision,c.step,c.completed,m.role,COALESCE((SELECT jsonb_agg(jsonb_build_object('mode',t.mode,'effective_from',t.effective_from::text,'revision',t.revision) ORDER BY t.revision) FROM company_tax_profile t WHERE t.company_id=c.id),'[]'::jsonb) AS tax_history FROM company c JOIN membership m ON m.company_id=c.id WHERE m.user_id=$1 AND c.deleted_at IS NULL";
async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    company: i64,
    actor: i64,
    rev: i64,
    action: &str,
    settings: &CompanySettings,
    tax_changed: bool,
) -> Result<()> {
    sqlx::query("INSERT INTO company_audit(company_id,revision,actor_id,action,settings) VALUES ($1,$2,$3,$4,$5)").bind(company).bind(rev).bind(actor).bind(action).bind(serde_json::to_value(settings).map_err(unavailable)?).execute(&mut **tx).await.map_err(unavailable)?;
    if tax_changed && !settings.tax_effective_from.is_empty() {
        sqlx::query("INSERT INTO company_tax_profile(company_id,revision,mode,effective_from) VALUES ($1,$2,$3,$4::text::date)").bind(company).bind(rev).bind(&settings.tax_mode).bind(&settings.tax_effective_from).execute(&mut **tx).await.map_err(unavailable)?;
    }
    Ok(())
}
#[async_trait]
impl CompanyStore for PgCompany {
    async fn delete(&self, actor: &str, company: &str, revision: i64) -> Result<()> {
        let actor = id(actor)?;
        let company = id(company)?;
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let row = sqlx::query("SELECT c.settings,c.revision,(c.deleted_at IS NOT NULL) AS deleted,m.role FROM company c JOIN membership m ON c.id=m.company_id WHERE c.id=$1 AND m.user_id=$2 FOR UPDATE OF c,m")
            .bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(CompanyError::NotFound)?;
        if row.get::<String, _>("role") != "owner" {
            return Err(CompanyError::Forbidden);
        }
        if row.get::<bool, _>("deleted") {
            return Ok(());
        }
        if row.get::<i64, _>("revision") != revision {
            return Err(CompanyError::Conflict);
        }
        let settings: CompanySettings =
            serde_json::from_value(row.get("settings")).map_err(unavailable)?;
        sqlx::query(
            "UPDATE company SET deleted_at=now(),updated_at=now(),revision=revision+1 WHERE id=$1",
        )
        .bind(company)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        audit(
            &mut tx,
            company,
            actor,
            revision + 1,
            "delete",
            &settings,
            false,
        )
        .await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(())
    }

    async fn list(&self, actor: &str) -> Result<Vec<Company>> {
        sqlx::query(&format!("{SELECT} ORDER BY c.id"))
            .bind(id(actor)?)
            .fetch_all(&self.0)
            .await
            .map_err(unavailable)?
            .into_iter()
            .map(map)
            .collect()
    }
    async fn get(&self, actor: &str, company: &str) -> Result<Company> {
        let row = sqlx::query(&format!("{SELECT} AND c.id=$2"))
            .bind(id(actor)?)
            .bind(id(company)?)
            .fetch_optional(&self.0)
            .await
            .map_err(unavailable)?
            .ok_or(CompanyError::NotFound)?;
        map(row)
    }
    async fn create(&self, actor: &str, key: &str, settings: CompanySettings) -> Result<Company> {
        let actor = id(actor)?;
        let data = serde_json::to_value(&settings).map_err(unavailable)?;
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        // Per-user serialization makes request replay safe, even with concurrent requests.
        sqlx::query("SELECT id FROM app_user WHERE id=$1 FOR UPDATE")
            .bind(actor)
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
        if let Some(row)=sqlx::query("SELECT id,initial_settings FROM company WHERE creator_id=$1 AND request_id=$2::text::uuid").bind(actor).bind(key).fetch_optional(&mut *tx).await.map_err(unavailable)? {
            if row.get::<serde_json::Value,_>("initial_settings") != data { return Err(CompanyError::Conflict); }
            let existing=row.get::<i64,_>("id").to_string();
            tx.commit().await.map_err(unavailable)?;
            return self.get(&actor.to_string(),&existing).await;
        }
        let company: i64=sqlx::query_scalar("INSERT INTO company(creator_id,request_id,initial_settings,settings) VALUES ($1,$2::text::uuid,$3,$3) RETURNING id").bind(actor).bind(key).bind(data).fetch_one(&mut *tx).await.map_err(unavailable)?;
        sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES ($1,$2,'owner')")
            .bind(company)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        audit(&mut tx, company, actor, 1, "create", &settings, true).await?;
        tx.commit().await.map_err(unavailable)?;
        self.get(&actor.to_string(), &company.to_string()).await
    }
    async fn update(
        &self,
        actor: &str,
        company: &str,
        revision: i64,
        step: i32,
        complete: bool,
        settings: CompanySettings,
    ) -> Result<Company> {
        let actor = id(actor)?;
        let company = id(company)?;
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let row=sqlx::query("SELECT c.settings,c.revision,c.completed,m.role FROM company c JOIN membership m ON c.id=m.company_id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR UPDATE OF c,m").bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(CompanyError::NotFound)?;
        if row.get::<String, _>("role") != "owner" {
            return Err(CompanyError::Forbidden);
        }
        if row.get::<i64, _>("revision") != revision {
            return Err(CompanyError::Conflict);
        }
        let completed = row.get::<bool, _>("completed") || complete;
        let issues = settings.issues(completed);
        if !issues.is_empty() {
            return Err(CompanyError::Invalid(issues));
        }
        let old: CompanySettings =
            serde_json::from_value(row.get("settings")).map_err(unavailable)?;
        sqlx::query("UPDATE company SET settings=$1,revision=revision+1,step=$2,completed=$3,updated_at=now() WHERE id=$4").bind(serde_json::to_value(&settings).map_err(unavailable)?).bind(step).bind(completed).bind(company).execute(&mut *tx).await.map_err(unavailable)?;
        audit(
            &mut tx,
            company,
            actor,
            revision + 1,
            if complete { "complete" } else { "update" },
            &settings,
            old.tax_mode != settings.tax_mode
                || old.tax_effective_from != settings.tax_effective_from,
        )
        .await?;
        tx.commit().await.map_err(unavailable)?;
        self.get(&actor.to_string(), &company.to_string()).await
    }
}

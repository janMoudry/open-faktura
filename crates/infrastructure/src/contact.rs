use async_trait::async_trait;
use faktura_application::contact::{
    Contact, ContactPage, ContactStore, Create, Error, Result, Search, Update,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgContact(pub PgPool);
fn unavailable(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(value: &str) -> Result<i64> {
    value.parse().map_err(|_| Error::NotFound)
}
fn map(row: sqlx::postgres::PgRow) -> Result<Contact> {
    Ok(Contact {
        id: row.get::<i64, _>("id").to_string(),
        company_id: row.get::<i64, _>("company_id").to_string(),
        revision: row.get("revision"),
        archived: row.get("archived"),
        data: serde_json::from_value(row.get("data")).map_err(unavailable)?,
    })
}
async fn authorize(
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
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    value: &Contact,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO contact_audit(company_id,contact_id,revision,actor_id,action,data,archived) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(id(&value.company_id)?).bind(id(&value.id)?).bind(value.revision).bind(actor).bind(action).bind(serde_json::to_value(&value.data).map_err(unavailable)?).bind(value.archived).execute(&mut **tx).await.map_err(unavailable)?;
    Ok(())
}
#[async_trait]
impl ContactStore for PgContact {
    async fn list(&self, actor: &str, company: &str, search: Search) -> Result<ContactPage> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
        let (_, company) = authorize(&mut tx, actor, company, false).await?;
        if search.tag_id.len() > 40
            || (!search.tag_id.is_empty()
                && search.tag_id.parse::<i64>().ok().is_none_or(|v| v <= 0))
        {
            return Err(Error::Invalid(vec!["tag_id".into()]));
        }
        let predicate = "company_id=$1 AND archived=$2 AND strpos(lower(concat_ws(' ', data->>'name',data->>'registration_number',data->>'email')),lower($3))>0 AND ($4='' OR EXISTS(SELECT 1 FROM document_tag_link l WHERE l.company_id=contact.company_id AND l.contact_id=contact.id AND l.tag_id::text=$4))";
        let total: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM contact WHERE {predicate}"))
                .bind(company)
                .bind(search.archived)
                .bind(&search.query)
                .bind(&search.tag_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(unavailable)?;
        let items = sqlx::query(&format!("SELECT * FROM contact WHERE {predicate} ORDER BY lower(data->>'name'),id LIMIT $5 OFFSET $6"))
            .bind(company).bind(search.archived).bind(&search.query).bind(&search.tag_id).bind(search.page_size).bind((search.page-1)*search.page_size).fetch_all(&mut *tx).await.map_err(unavailable)?.into_iter().map(map).collect::<Result<Vec<_>>>()?;
        tx.commit().await.map_err(unavailable)?;
        Ok(ContactPage {
            items,
            total,
            page: search.page,
            page_size: search.page_size,
        })
    }
    async fn get(&self, actor: &str, company: &str, contact: &str) -> Result<Contact> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (_, company) = authorize(&mut tx, actor, company, false).await?;
        let value = map(
            sqlx::query("SELECT * FROM contact WHERE company_id=$1 AND id=$2")
                .bind(company)
                .bind(id(contact)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(unavailable)?
                .ok_or(Error::NotFound)?,
        )?;
        tx.commit().await.map_err(unavailable)?;
        Ok(value)
    }
    async fn create(&self, actor: &str, company: &str, command: Create) -> Result<Contact> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (actor, company) = authorize(&mut tx, actor, company, true).await?;
        let data = serde_json::to_value(&command.data).map_err(unavailable)?;
        let inserted = sqlx::query("INSERT INTO contact(company_id,creator_id,request_id,initial_data,data) VALUES ($1,$2,$3::text::uuid,$4,$4) ON CONFLICT(company_id,request_id) DO NOTHING RETURNING *")
            .bind(company).bind(actor).bind(&command.request_id).bind(&data).fetch_optional(&mut *tx).await.map_err(unavailable)?;
        let value = if let Some(row) = inserted {
            let value = map(row)?;
            audit(&mut tx, actor, &value, "create").await?;
            value
        } else {
            let row = sqlx::query(
                "SELECT * FROM contact WHERE company_id=$1 AND request_id=$2::text::uuid",
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
    ) -> Result<Contact> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (actor, company) = authorize(&mut tx, actor, company, true).await?;
        let contact = id(contact)?;
        let previous = map(sqlx::query(
            "SELECT * FROM contact WHERE company_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(company)
        .bind(contact)
        .fetch_optional(&mut *tx)
        .await
        .map_err(unavailable)?
        .ok_or(Error::NotFound)?)?;
        if previous.revision != command.revision {
            return Err(Error::Conflict);
        }
        let action = match (previous.archived, command.archived) {
            (false, true) => "archive",
            (true, false) => "restore",
            _ => "update",
        };
        let value = map(sqlx::query("UPDATE contact SET data=$3,archived=$4,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *")
            .bind(company).bind(contact).bind(serde_json::to_value(command.data).map_err(unavailable)?).bind(command.archived).fetch_one(&mut *tx).await.map_err(unavailable)?)?;
        audit(&mut tx, actor, &value, action).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(value)
    }
}

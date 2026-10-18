use async_trait::async_trait;
use faktura_application::document_tags::*;
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgTags(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    write: bool,
) -> Result<(i64, i64)> {
    let actor = id(actor)?;
    let company = id(company)?;
    let role:Option<String>=sqlx::query_scalar("SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR UPDATE OF c").bind(company).bind(actor).fetch_optional(&mut **tx).await.map_err(err)?;
    let role = role.ok_or(Error::NotFound)?;
    if write && role == "viewer" {
        return Err(Error::Forbidden);
    }
    Ok((actor, company))
}
async fn target(tx: &mut Transaction<'_, Postgres>, company: i64, t: &Target) -> Result<i64> {
    let target = id(&t.id)?;
    let query = format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE company_id=$1 AND id=$2)",
        t.kind.table()
    );
    let exists: bool = sqlx::query_scalar(&query)
        .bind(company)
        .bind(target)
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    if !exists {
        return Err(Error::NotFound);
    }
    Ok(target)
}
async fn tags(
    tx: &mut Transaction<'_, Postgres>,
    company: i64,
    t: Option<Target>,
) -> Result<Vec<Tag>> {
    let (filter, target_id) = if let Some(t) = t {
        let target_id = target(tx, company, &t).await?;
        (format!(" AND EXISTS(SELECT 1 FROM document_tag_link l WHERE l.company_id=t.company_id AND l.tag_id=t.id AND l.{}=$2)",t.kind.column()),target_id)
    } else {
        (" AND $2::bigint=0".into(), 0)
    };
    let query=format!("SELECT t.id,t.name,t.revision,(SELECT count(*) FROM document_tag_link l WHERE l.company_id=t.company_id AND l.tag_id=t.id) usage_count FROM document_tag t WHERE t.company_id=$1{filter} ORDER BY lower(t.name),t.id");
    Ok(sqlx::query(&query)
        .bind(company)
        .bind(target_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(err)?
        .into_iter()
        .map(|r| Tag {
            id: r.get::<i64, _>("id").to_string(),
            name: r.get("name"),
            revision: r.get("revision"),
            usage_count: r.get("usage_count"),
        })
        .collect())
}
#[async_trait]
impl TagStore for PgTags {
    async fn list(&self, actor: &str, company: &str, t: Option<Target>) -> Result<Vec<Tag>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, company) = authorize(&mut tx, actor, company, false).await?;
        let result = tags(&mut tx, company, t).await?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn execute(&self, actor: &str, company: &str, command: Command) -> Result<Vec<Tag>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (actor, company) = authorize(&mut tx, actor, company, true).await?;
        if uuid::Uuid::parse_str(&command.request_id).is_err() {
            return Err(Error::Invalid(vec!["request_id".into()]));
        }
        let input = serde_json::to_value(&command).map_err(err)?;
        if let Some(row)=sqlx::query("SELECT actor_id,input,result FROM document_tag_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(company).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)?{
   if row.get::<i64,_>("actor_id")!=actor||row.get::<serde_json::Value,_>("input")!=input{return Err(Error::Conflict)}return serde_json::from_value(row.get("result")).map_err(err)
  }
        match command.action {
            Action::Create { name } | Action::Rename { name, .. }
                if name.trim().is_empty() || name.trim().chars().count() > 80 =>
            {
                return Err(Error::Invalid(vec!["name".into()]))
            }
            Action::Create { name } => {
                let count=sqlx::query("INSERT INTO document_tag(company_id,name) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(company).bind(name.trim()).execute(&mut *tx).await.map_err(err)?.rows_affected();
                if count != 1 {
                    return Err(Error::Conflict);
                }
            }
            Action::Rename {
                tag_id,
                revision,
                name,
            } => {
                let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM document_tag WHERE company_id=$1 AND lower(name)=lower($2) AND id<>$3)").bind(company).bind(name.trim()).bind(id(&tag_id)?).fetch_one(&mut *tx).await.map_err(err)?;
                if duplicate {
                    return Err(Error::Conflict);
                }
                let count=sqlx::query("UPDATE document_tag SET name=$4,revision=revision+1 WHERE company_id=$1 AND id=$2 AND revision=$3").bind(company).bind(id(&tag_id)?).bind(revision).bind(name.trim()).execute(&mut *tx).await.map_err(err)?.rows_affected();
                if count != 1 {
                    return Err(Error::Conflict);
                }
            }
            Action::Delete { tag_id, revision } => {
                let count = sqlx::query(
                    "DELETE FROM document_tag WHERE company_id=$1 AND id=$2 AND revision=$3",
                )
                .bind(company)
                .bind(id(&tag_id)?)
                .bind(revision)
                .execute(&mut *tx)
                .await
                .map_err(err)?
                .rows_affected();
                if count != 1 {
                    return Err(Error::Conflict);
                }
            }
            Action::Attach { tag_id, target: t } | Action::Detach { tag_id, target: t } => {
                let tag = id(&tag_id)?;
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM document_tag WHERE company_id=$1 AND id=$2)",
                )
                .bind(company)
                .bind(tag)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
                if !exists {
                    return Err(Error::NotFound);
                }
                let tid = target(&mut tx, company, &t).await?;
                let query = if input["action"] == "attach" {
                    format!("INSERT INTO document_tag_link(company_id,tag_id,{}) VALUES($1,$2,$3) ON CONFLICT DO NOTHING",t.kind.column())
                } else {
                    format!(
                        "DELETE FROM document_tag_link WHERE company_id=$1 AND tag_id=$2 AND {}=$3",
                        t.kind.column()
                    )
                };
                sqlx::query(&query)
                    .bind(company)
                    .bind(tag)
                    .bind(tid)
                    .execute(&mut *tx)
                    .await
                    .map_err(err)?;
            }
        }
        let result = tags(&mut tx, company, None).await?;
        sqlx::query("INSERT INTO document_tag_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(company).bind(command.request_id).bind(actor).bind(input).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

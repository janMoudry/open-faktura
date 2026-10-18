use async_trait::async_trait;
use faktura_application::document_exports::*;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgDocumentExports(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn file(id: &str, content: serde_json::Value) -> Result<File> {
    Ok(File {
        bytes: serde_json::to_vec_pretty(&content).map_err(err)?,
        filename: format!("document-export-{id}.json"),
    })
}
async fn source(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    target: &Target,
) -> Result<serde_json::Value> {
    let query=format!("SELECT to_jsonb(t)-'pdf'-'initial_data' FROM {} t WHERE company_id=$1 AND id=$2 FOR UPDATE OF t",target.kind.table());
    let value = sqlx::query_scalar(&query)
        .bind(c)
        .bind(id(&target.id)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    if matches!(target.kind, Kind::Invoice) {
        let trashed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invoice_trash WHERE company_id=$1 AND draft_id=$2 AND deleted_at IS NOT NULL)").bind(c).bind(id(&target.id)?).fetch_one(&mut **tx).await.map_err(err)?;
        if trashed {
            return Err(Error::Conflict);
        }
    }
    if matches!(target.kind, Kind::Sales) {
        crate::sales::ensure_not_trashed(tx, c, id(&target.id)?).await?;
    }
    Ok(value)
}
async fn history(tx: &mut Transaction<'_, Postgres>, c: i64, target: Target) -> Result<History> {
    source(tx, c, &target).await?;
    let rows=sqlx::query("SELECT id::text,actor_id,sha256,created_at,input FROM document_export WHERE company_id=$1 AND input->'targets' @> $2::jsonb ORDER BY created_at DESC,id DESC LIMIT 100").bind(c).bind(serde_json::json!([{"kind":target.kind,"id":target.id}])).fetch_all(&mut **tx).await.map_err(err)?;
    let exports = rows
        .into_iter()
        .map(|r| {
            let input: Request = serde_json::from_value(r.get("input")).map_err(err)?;
            Ok(Record {
                id: r.get("id"),
                actor_id: r.get::<i64, _>("actor_id").to_string(),
                sha256: r.get("sha256"),
                created_at: r
                    .get::<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                targets: input.targets,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let locked_by_export:Option<String>=sqlx::query_scalar("SELECT export_id::text FROM document_export_lock WHERE company_id=$1 AND kind=$2 AND document_id=$3").bind(c).bind(target.kind.as_str()).bind(id(&target.id)?).fetch_optional(&mut **tx).await.map_err(err)?;
    Ok(History {
        exports,
        locked_by_export,
    })
}
#[async_trait]
impl ExportStore for PgDocumentExports {
    async fn history(&self, actor: &str, company: &str, target: Target) -> Result<History> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, actor, company, false).await?;
        let result = history(&mut tx, c, target).await?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn file(&self, actor: &str, company: &str, input: &str) -> Result<File> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, actor, company, false).await?;
        let export = uuid::Uuid::parse_str(input).map_err(|_| Error::NotFound)?;
        let content: serde_json::Value = sqlx::query_scalar(
            "SELECT content FROM document_export WHERE company_id=$1 AND id=$2::text::uuid",
        )
        .bind(c)
        .bind(export.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
        let result = file(input, content)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn create(&self, actor: &str, company: &str, request: Request) -> Result<File> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, actor, company, request.lock).await?;
        if uuid::Uuid::parse_str(&request.request_id).is_err()
            || request.targets.is_empty()
            || request.targets.len() > 100
        {
            return Err(Error::Invalid(vec!["request_id".into(), "targets".into()]));
        }
        // Serialize by company before acquiring individual document locks. All export
        // and unlock operations use this order, so reverse input order cannot deadlock.
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&request).map_err(err)?;
        if let Some(row)=sqlx::query("SELECT id::text,actor_id,input,content FROM document_export WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&request.request_id).fetch_optional(&mut *tx).await.map_err(err)?{
   if row.get::<i64,_>("actor_id")!=a||row.get::<serde_json::Value,_>("input")!=input{return Err(Error::Conflict)}return file(&row.get::<String,_>("id"),row.get("content"))
  }
        let mut seen = std::collections::HashSet::new();
        let mut documents = Vec::new();
        let mut size = 0;
        for target in &request.targets {
            if !seen.insert((target.kind.as_str(), id(&target.id)?)) {
                return Err(Error::Invalid(vec!["targets".into()]));
            }
            let value = source(
                &mut tx,
                c,
                &Target {
                    kind: target.kind.clone(),
                    id: target.id.clone(),
                },
            )
            .await?;
            if value["revision"].as_i64() != Some(target.revision) {
                return Err(Error::Conflict);
            }
            size += serde_json::to_vec(&value).map_err(err)?.len();
            if size > 20 * 1024 * 1024 {
                return Err(Error::Invalid(vec!["export_limit".into()]));
            }
            documents.push(serde_json::json!({"kind":target.kind,"id":target.id,"revision":target.revision,"snapshot":value}));
        }
        let export = uuid::Uuid::new_v4().to_string();
        let content = serde_json::json!({"format":"open-faktura-documents","version":1,"company_id":company,"documents":documents});
        let result = file(&export, content.clone())?;
        let hash = format!("{:x}", Sha256::digest(&result.bytes));
        sqlx::query("INSERT INTO document_export(id,company_id,request_id,actor_id,input,content,sha256) VALUES($1::text::uuid,$2,$3::text::uuid,$4,$5,$6,$7)").bind(&export).bind(c).bind(&request.request_id).bind(a).bind(&input).bind(content).bind(hash).execute(&mut *tx).await.map_err(err)?;
        if request.lock {
            for target in request.targets {
                sqlx::query("INSERT INTO document_export_lock(company_id,kind,document_id,export_id) VALUES($1,$2,$3,$4::text::uuid) ON CONFLICT DO NOTHING").bind(c).bind(target.kind.as_str()).bind(id(&target.id)?).bind(&export).execute(&mut *tx).await.map_err(err)?;
            }
        }
        sqlx::query("INSERT INTO document_export_event(company_id,request_id,actor_id,action,input,result) VALUES($1,$2::text::uuid,$3,'export',$4,$5)").bind(c).bind(request.request_id).bind(a).bind(input).bind(serde_json::json!({"export_id":export})).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn unlock(&self, actor: &str, company: &str, command: Unlock) -> Result<History> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, actor, company, true).await?;
        let owner: bool = sqlx::query_scalar(
            "SELECT role='owner' FROM membership WHERE company_id=$1 AND user_id=$2",
        )
        .bind(c)
        .bind(a)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if !owner {
            return Err(Error::Forbidden);
        }
        if uuid::Uuid::parse_str(&command.request_id).is_err()
            || uuid::Uuid::parse_str(&command.export_id).is_err()
            || command.reason.trim().is_empty()
            || command.reason.chars().count() > 500
        {
            return Err(Error::Invalid(vec!["reason".into(), "request_id".into()]));
        }
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&command).map_err(err)?;
        if let Some(row)=sqlx::query("SELECT actor_id,input,result FROM document_export_event WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)?{if row.get::<i64,_>("actor_id")!=a||row.get::<serde_json::Value,_>("input")!=input{return Err(Error::Conflict)}return serde_json::from_value(row.get("result")).map_err(err)}
        source(&mut tx, c, &command.target).await?;
        let count=sqlx::query("DELETE FROM document_export_lock WHERE company_id=$1 AND kind=$2 AND document_id=$3 AND export_id=$4::text::uuid").bind(c).bind(command.target.kind.as_str()).bind(id(&command.target.id)?).bind(&command.export_id).execute(&mut *tx).await.map_err(err)?.rows_affected();
        if count != 1 {
            return Err(Error::Conflict);
        }
        let result = history(&mut tx, c, command.target).await?;
        sqlx::query("INSERT INTO document_export_event(company_id,request_id,actor_id,action,input,result,reason) VALUES($1,$2::text::uuid,$3,'unlock',$4,$5,$6)").bind(c).bind(command.request_id).bind(a).bind(input).bind(serde_json::to_value(&result).map_err(err)?).bind(command.reason.trim()).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

use async_trait::async_trait;
use faktura_application::draft_trash::*;
use sqlx::{PgPool, Row};
pub struct PgTrash(pub PgPool);
fn err(e: sqlx::Error) -> Error {
    if e.as_database_error()
        .is_some_and(|e| e.code().as_deref() == Some("23505"))
    {
        Error::Conflict
    } else {
        Error::Unavailable
    }
}
fn encode_err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn entry(r: sqlx::postgres::PgRow) -> Entry {
    let data: serde_json::Value = r.get("data");
    Entry {
        id: r.get::<i64, _>("id").to_string(),
        title: data["calculation"]["lines"][0]["description"]
            .as_str()
            .unwrap_or_default()
            .into(),
        customer: data["customer"]["name"].as_str().unwrap_or_default().into(),
        revision: r.get("revision"),
        deleted_at: r
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
            .map(|v| v.to_rfc3339()),
    }
}
#[async_trait]
impl TrashStore for PgTrash {
    async fn list(&self, a: &str, c: &str, page: i64) -> Result<Page> {
        if !(1..=100000).contains(&page) {
            return Err(Error::Invalid(vec!["page".into()]));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let total = sqlx::query_scalar(
            "SELECT count(*) FROM invoice_trash WHERE company_id=$1 AND deleted_at IS NOT NULL",
        )
        .bind(c)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items=sqlx::query("SELECT d.id,d.data,d.revision,t.deleted_at FROM invoice_trash t JOIN invoice_draft d ON d.company_id=t.company_id AND d.id=t.draft_id WHERE t.company_id=$1 AND t.deleted_at IS NOT NULL ORDER BY t.deleted_at DESC,d.id DESC LIMIT 25 OFFSET $2").bind(c).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(entry).collect();
        tx.commit().await.map_err(err)?;
        Ok(Page {
            items,
            total,
            page,
            page_size: 25,
        })
    }
    async fn execute(&self, a: &str, c: &str, command: Command) -> Result<Entry> {
        uuid::Uuid::parse_str(&command.request_id)
            .map_err(|_| Error::Invalid(vec!["request_id".into()]))?;
        let id: i64 = command.id.parse().map_err(|_| Error::NotFound)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        // Lock the invoice before looking up replay: concurrent retries serialize here.
        let row =
            sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(c)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?;
        let input = serde_json::to_value(&command).map_err(encode_err)?;
        if let Some(old)=sqlx::query("SELECT actor_id,command,result FROM invoice_trash_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)? {
   if old.get::<i64,_>("actor_id")!=a || old.get::<serde_json::Value,_>("command")!=input{return Err(Error::Conflict);}
   return serde_json::from_value(old.get("result")).map_err(encode_err);
  }
        if row.get::<Option<String>, _>("number").is_some()
            || row.get::<i64, _>("revision") != command.revision
        {
            return Err(Error::Conflict);
        }
        let linked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM advance_settlement_link WHERE company_id=$1 AND invoice_id=$2) OR EXISTS(SELECT 1 FROM sales_document WHERE company_id=$1 AND invoice_id=$2) OR EXISTS(SELECT 1 FROM recurring_occurrence WHERE company_id=$1 AND invoice_id=$2) OR EXISTS(SELECT 1 FROM document_export_lock WHERE company_id=$1 AND kind='invoice' AND document_id=$2)").bind(c).bind(id).fetch_one(&mut *tx).await.map_err(err)?;
        if linked {
            return Err(Error::Conflict);
        }
        let deleted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invoice_trash WHERE company_id=$1 AND draft_id=$2 AND deleted_at IS NOT NULL)").bind(c).bind(id).fetch_one(&mut *tx).await.map_err(err)?;
        let trash = matches!(command.action, Action::Trash);
        if deleted == trash {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO invoice_trash(company_id,draft_id,deleted_at) VALUES($1,$2,CASE WHEN $3 THEN now() ELSE NULL END) ON CONFLICT(company_id,draft_id) DO UPDATE SET deleted_at=excluded.deleted_at").bind(c).bind(id).bind(trash).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE invoice_draft SET revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(c).bind(id).execute(&mut *tx).await.map_err(err)?;
        let result=entry(sqlx::query("SELECT d.id,d.data,d.revision,t.deleted_at FROM invoice_draft d JOIN invoice_trash t ON t.company_id=d.company_id AND t.draft_id=d.id WHERE d.company_id=$1 AND d.id=$2").bind(c).bind(id).fetch_one(&mut *tx).await.map_err(err)?);
        sqlx::query("INSERT INTO invoice_trash_command(company_id,request_id,actor_id,draft_id,command,result) VALUES($1,$2::text::uuid,$3,$4,$5,$6)").bind(c).bind(&command.request_id).bind(a).bind(id).bind(input).bind(serde_json::to_value(&result).map_err(encode_err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

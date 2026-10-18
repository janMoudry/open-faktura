use async_trait::async_trait;
use faktura_application::sales_trash::*;
use sqlx::{PgPool, Row};
pub struct PgSalesTrash(pub PgPool);
fn err(e: sqlx::Error) -> Error {
    if e.as_database_error()
        .is_some_and(|e| matches!(e.code().as_deref(), Some("23505" | "23514")))
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
        title: data["name"].as_str().unwrap_or_default().into(),
        customer: data["invoice"]["customer"]["name"]
            .as_str()
            .unwrap_or_default()
            .into(),
        kind: r.get("kind"),
        revision: r.get("revision"),
        deleted_at: r
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
            .map(|v| v.to_rfc3339()),
    }
}
#[async_trait]
impl TrashStore for PgSalesTrash {
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
            "SELECT count(*) FROM sales_trash WHERE company_id=$1 AND deleted_at IS NOT NULL",
        )
        .bind(c)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items=sqlx::query("SELECT d.id,d.kind,d.data,d.revision,t.deleted_at FROM sales_trash t JOIN sales_document d ON d.company_id=t.company_id AND d.id=t.document_id WHERE t.company_id=$1 AND t.deleted_at IS NOT NULL ORDER BY t.deleted_at DESC,d.id DESC LIMIT 25 OFFSET $2").bind(c).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(entry).collect();
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
        // Lock the source before looking up replay: concurrent retries serialize here.
        let row =
            sqlx::query("SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(c)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?;
        let input = serde_json::to_value(&command).map_err(encode_err)?;
        if let Some(old)=sqlx::query("SELECT actor_id,command,result FROM sales_trash_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)? {
   if old.get::<i64,_>("actor_id")!=a || old.get::<serde_json::Value,_>("command")!=input{return Err(Error::Conflict);}
   return serde_json::from_value(old.get("result")).map_err(encode_err);
  }
        if !["quote", "template"].contains(&row.get::<String, _>("kind").as_str())
            || row.get::<i64, _>("revision") != command.revision
        {
            return Err(Error::Conflict);
        }
        // Source lock serializes conversion, publication, delivery and export.
        // Check linked tables afterwards so a waiter sees their committed history.
        let linked:bool=sqlx::query_scalar("SELECT $3::bigint IS NOT NULL OR EXISTS(SELECT 1 FROM sales_conversion WHERE company_id=$1 AND document_id=$2) OR EXISTS(SELECT 1 FROM quote_delivery WHERE company_id=$1 AND document_id=$2) OR EXISTS(SELECT 1 FROM quote_share WHERE company_id=$1 AND document_id=$2) OR EXISTS(SELECT 1 FROM document_export_lock WHERE company_id=$1 AND kind='sales' AND document_id=$2)").bind(c).bind(id).bind(row.get::<Option<i64>,_>("invoice_id")).fetch_one(&mut *tx).await.map_err(err)?;
        if linked {
            return Err(Error::Conflict);
        }
        let deleted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sales_trash WHERE company_id=$1 AND document_id=$2 AND deleted_at IS NOT NULL)").bind(c).bind(id).fetch_one(&mut *tx).await.map_err(err)?;
        let trash = matches!(command.action, Action::Trash);
        if deleted == trash {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO sales_trash(company_id,document_id,deleted_at) VALUES($1,$2,CASE WHEN $3 THEN now() ELSE NULL END) ON CONFLICT(company_id,document_id) DO UPDATE SET deleted_at=excluded.deleted_at").bind(c).bind(id).bind(trash).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE sales_document SET revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(c).bind(id).execute(&mut *tx).await.map_err(err)?;
        let result=entry(sqlx::query("SELECT d.id,d.kind,d.data,d.revision,t.deleted_at FROM sales_document d JOIN sales_trash t ON t.company_id=d.company_id AND t.document_id=d.id WHERE d.company_id=$1 AND d.id=$2").bind(c).bind(id).fetch_one(&mut *tx).await.map_err(err)?);
        let snapshot = crate::sales::map(
            sqlx::query("SELECT * FROM sales_document WHERE company_id=$1 AND id=$2")
                .bind(c)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?,
        )?;
        crate::sales::audit(
            &mut tx,
            a,
            &snapshot,
            if trash { "trash" } else { "restore" },
        )
        .await?;
        sqlx::query("INSERT INTO sales_trash_command(company_id,request_id,actor_id,document_id,command,result) VALUES($1,$2::text::uuid,$3,$4,$5,$6)").bind(c).bind(&command.request_id).bind(a).bind(id).bind(input).bind(serde_json::to_value(&result).map_err(encode_err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

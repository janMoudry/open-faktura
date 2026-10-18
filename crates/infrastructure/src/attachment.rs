use async_trait::async_trait;
use base64::Engine;
use faktura_application::attachment::*;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgAttachment(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(s: &str) -> Result<i64> {
    s.parse().map_err(|_| Error::NotFound)
}
fn column(k: &str) -> Result<&'static str> {
    match k {
        "invoice" => Ok("invoice_id"),
        "received" => Ok("received_id"),
        "quote" => Ok("quote_id"),
        _ => Err(Error::NotFound),
    }
}
fn delivery_column(k: &str) -> Result<&'static str> {
    match k {
        "invoice" => Ok("invoice_delivery_id"),
        "quote" => Ok("quote_delivery_id"),
        _ => Err(Error::NotFound),
    }
}
fn map(row: &sqlx::postgres::PgRow) -> Attachment {
    Attachment {
        id: row.get::<i64, _>("id").to_string(),
        filename: row.get("filename"),
        media_type: row.get("media_type"),
        size: row.get("size"),
        sha256: row.get("sha256"),
        created_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
    }
}
async fn document(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    k: &str,
    d: i64,
    write: bool,
) -> Result<()> {
    let query = match k {
        "received" if write => "SELECT id FROM received_invoice WHERE company_id=$1 AND id=$2 AND NOT cancelled FOR SHARE",
        "received" => "SELECT id FROM received_invoice WHERE company_id=$1 AND id=$2 FOR SHARE",
        "invoice" => "SELECT id FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR SHARE",
        "quote" if write => "SELECT id FROM sales_document WHERE company_id=$1 AND id=$2 AND kind='quote' AND NOT archived FOR SHARE",
        "quote" => "SELECT id FROM sales_document WHERE company_id=$1 AND id=$2 AND kind='quote' FOR SHARE",
        _ => return Err(Error::NotFound),
    };
    sqlx::query_scalar::<_, i64>(query)
        .bind(c)
        .bind(d)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    if k == "quote" {
        crate::sales::ensure_not_trashed(tx, c, d).await?;
    }
    Ok(())
}
fn media(name: &str, bytes: &[u8]) -> Result<&'static str> {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let value = match ext.as_str() {
        "pdf" if bytes.starts_with(b"%PDF-") => "application/pdf",
        "png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => "image/png",
        "jpg" | "jpeg" if bytes.starts_with(b"\xff\xd8\xff") => "image/jpeg",
        "txt" if std::str::from_utf8(bytes).is_ok() && !bytes.contains(&0) => "text/plain",
        "rar" if bytes.starts_with(b"Rar!\x1a\x07") => "application/vnd.rar",
        "zip" if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") => {
            "application/zip"
        }
        "docx" | "xlsx" | "odt" | "ods" => {
            let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
                .map_err(|_| Error::Invalid(vec!["file_type".into()]))?;
            if archive.len() > 4096 {
                return Err(Error::Invalid(vec!["file_type".into()]));
            }
            let names = archive.file_names().collect::<Vec<_>>();
            match ext.as_str() {
                "docx" if names.contains(&"word/document.xml") => {
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                }
                "xlsx" if names.contains(&"xl/workbook.xml") => {
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                }
                "odt" if names.contains(&"content.xml") && names.contains(&"mimetype") => {
                    "application/vnd.oasis.opendocument.text"
                }
                "ods" if names.contains(&"content.xml") && names.contains(&"mimetype") => {
                    "application/vnd.oasis.opendocument.spreadsheet"
                }
                _ => return Err(Error::Invalid(vec!["file_type".into()])),
            }
        }
        _ => return Err(Error::Invalid(vec!["file_type".into()])),
    };
    Ok(value)
}
#[async_trait]
impl AttachmentStore for PgAttachment {
    async fn list(&self, a: &str, c: &str, k: &str, d: &str) -> Result<Vec<Attachment>> {
        let col = column(k)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        document(&mut tx, c, k, id(d)?, false).await?;
        let rows=sqlx::query(&format!("SELECT id,filename,media_type,size,sha256,created_at FROM document_attachment WHERE company_id=$1 AND {col}=$2 AND deleted_at IS NULL ORDER BY id")).bind(c).bind(id(d)?).fetch_all(&mut *tx).await.map_err(err)?;
        Ok(rows.iter().map(map).collect())
    }
    async fn upload(&self, a: &str, c: &str, k: &str, d: &str, cmd: Upload) -> Result<Attachment> {
        let col = column(k)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (actor, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&cmd.base64)
            .map_err(|_| Error::Invalid(vec!["file".into()]))?;
        if bytes.is_empty() || bytes.len() > 2097152 {
            return Err(Error::Invalid(vec!["file_size".into()]));
        }
        let mime = media(&cmd.filename, &bytes)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        document(&mut tx, c, k, id(d)?, true).await?;
        if let Some(row) = sqlx::query(
            "SELECT * FROM document_attachment WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if row.get::<Option<i64>, _>(col) != Some(id(d)?)
                || row.get::<i64, _>("actor_id") != actor
                || row.get::<String, _>("filename") != cmd.filename
                || row.get::<String, _>("sha256") != hash
                || row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
                    .is_some()
            {
                return Err(Error::Conflict);
            }
            return Ok(map(&row));
        }
        let count:i64=sqlx::query_scalar(&format!("SELECT count(*) FROM document_attachment WHERE company_id=$1 AND {col}=$2 AND deleted_at IS NULL")).bind(c).bind(id(d)?).fetch_one(&mut *tx).await.map_err(err)?;
        if count >= 5 {
            return Err(Error::Invalid(vec!["file_count".into()]));
        }
        let used:i64=sqlx::query_scalar("SELECT COALESCE(sum(octet_length(content)),0)::bigint FROM document_attachment WHERE company_id=$1").bind(c).fetch_one(&mut *tx).await.map_err(err)?;
        if used + bytes.len() as i64 > 268435456 {
            return Err(Error::Invalid(vec!["storage_limit".into()]));
        }
        let row=sqlx::query(&format!("INSERT INTO document_attachment(company_id,{col},request_id,actor_id,filename,media_type,size,sha256,content) VALUES($1,$2,$3::text::uuid,$4,$5,$6,$7,$8,$9) RETURNING *")).bind(c).bind(id(d)?).bind(cmd.request_id).bind(actor).bind(cmd.filename).bind(mime).bind(bytes.len() as i64).bind(hash).bind(bytes).fetch_one(&mut *tx).await.map_err(err)?;
        let file = map(&row);
        audit(&mut tx, c, id(&file.id)?, actor, "uploaded").await?;
        tx.commit().await.map_err(err)?;
        Ok(file)
    }
    async fn download(
        &self,
        a: &str,
        c: &str,
        k: &str,
        d: &str,
        file: &str,
        delivery: Option<&str>,
    ) -> Result<File> {
        let col = column(k)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        document(&mut tx, c, k, id(d)?, false).await?;
        let predicate = if let Some(delivery) = delivery {
            let dc = delivery_column(k)?;
            let table = if k == "invoice" {
                "invoice_delivery"
            } else {
                "quote_delivery"
            };
            let valid:bool=sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM delivery_attachment a JOIN {table} d ON d.company_id=a.company_id AND d.id=a.{dc} WHERE a.company_id=$1 AND a.attachment_id=$2 AND d.id=$3 AND d.document_id=$4)")).bind(c).bind(id(file)?).bind(id(delivery)?).bind(id(d)?).fetch_one(&mut *tx).await.map_err(err)?;
            if !valid {
                return Err(Error::NotFound);
            }
            "true"
        } else {
            "deleted_at IS NULL"
        };
        let row=sqlx::query(&format!("SELECT * FROM document_attachment WHERE company_id=$1 AND {col}=$2 AND id=$3 AND {predicate} AND content IS NOT NULL")).bind(c).bind(id(d)?).bind(id(file)?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        Ok(File {
            metadata: map(&row),
            content: row.get("content"),
        })
    }
    async fn delete(&self, a: &str, c: &str, k: &str, d: &str, file: &str) -> Result<()> {
        let col = column(k)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (actor, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        document(&mut tx, c, k, id(d)?, true).await?;
        let row=sqlx::query(&format!("SELECT deleted_at FROM document_attachment WHERE company_id=$1 AND {col}=$2 AND id=$3 FOR UPDATE")).bind(c).bind(id(d)?).bind(id(file)?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        if row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")
            .is_none()
        {
            sqlx::query("UPDATE document_attachment SET deleted_at=now(),content=CASE WHEN EXISTS(SELECT 1 FROM delivery_attachment WHERE company_id=$1 AND attachment_id=$2) THEN content ELSE NULL END WHERE company_id=$1 AND id=$2").bind(c).bind(id(file)?).execute(&mut *tx).await.map_err(err)?;
            audit(&mut tx, c, id(file)?, actor, "deleted").await?;
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    f: i64,
    a: i64,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO attachment_audit(company_id,attachment_id,actor_id,action) VALUES($1,$2,$3,$4)").bind(c).bind(f).bind(a).bind(action).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
// Caller holds the same company advisory lock as upload/delete.
pub(crate) async fn snapshot(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    k: &str,
    d: i64,
    ids: &[String],
) -> Result<Vec<Attachment>> {
    let col = column(k)?;
    let mut files = Vec::new();
    for file in ids {
        let row=sqlx::query(&format!("SELECT id,filename,media_type,size,sha256,created_at FROM document_attachment WHERE company_id=$1 AND {col}=$2 AND id=$3 AND deleted_at IS NULL AND content IS NOT NULL FOR SHARE")).bind(c).bind(d).bind(id(file)?).fetch_optional(&mut **tx).await.map_err(err)?.ok_or(Error::Conflict)?;
        files.push(map(&row));
    }
    Ok(files)
}
pub(crate) async fn link(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    k: &str,
    delivery: i64,
    files: &[Attachment],
) -> Result<()> {
    let col = delivery_column(k)?;
    for file in files {
        sqlx::query(&format!(
            "INSERT INTO delivery_attachment(company_id,{col},attachment_id) VALUES($1,$2,$3)"
        ))
        .bind(c)
        .bind(delivery)
        .bind(id(&file.id)?)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    }
    Ok(())
}
pub(crate) async fn queued(pool: &PgPool, c: i64, k: &str, delivery: i64) -> Result<Vec<File>> {
    let col = delivery_column(k)?;
    let rows=sqlx::query(&format!("SELECT f.* FROM document_attachment f JOIN delivery_attachment d ON d.company_id=f.company_id AND d.attachment_id=f.id WHERE d.company_id=$1 AND d.{col}=$2 ORDER BY f.id")).bind(c).bind(delivery).fetch_all(pool).await.map_err(err)?;
    rows.iter()
        .map(|r| {
            Ok(File {
                metadata: map(r),
                content: r
                    .get::<Option<Vec<u8>>, _>("content")
                    .ok_or(Error::Unavailable)?,
            })
        })
        .collect()
}

use async_trait::async_trait;
use base64::Engine;
use faktura_application::inbox::*;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgInbox(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn invalid() -> Error {
    Error::Invalid(vec!["inbox".into()])
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn map(r: &sqlx::postgres::PgRow) -> Document {
    Document {
        id: r.get::<i64, _>("id").to_string(),
        revision: r.get("revision"),
        filename: r.get("filename"),
        media_type: r.get("media_type"),
        size: r.get("size"),
        sha256: r.get("sha256"),
        archived: r.get("archived"),
        received_kind: r.try_get("received_kind").unwrap_or(None),
        received_id: r
            .get::<Option<i64>, _>("received_id")
            .map(|v| v.to_string()),
        created_at: r
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
    }
}
async fn load(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    i: &str,
) -> Result<sqlx::postgres::PgRow> {
    sqlx::query("SELECT i.*,(SELECT kind FROM received_invoice r WHERE r.company_id=i.company_id AND r.id=i.received_id) AS received_kind FROM inbox_document i WHERE i.company_id=$1 AND i.id=$2 FOR UPDATE")
        .bind(c)
        .bind(id(i)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)
}
async fn lock(tx: &mut Transaction<'_, Postgres>, a: &str, c: &str) -> Result<(i64, i64)> {
    let (a, c) = crate::draft::authorize(tx, a, c, true).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok((a, c))
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    k: &str,
    input: &serde_json::Value,
) -> Result<Option<Document>> {
    uuid::Uuid::parse_str(k).map_err(|_| invalid())?;
    if let Some(r) =
        sqlx::query("SELECT * FROM inbox_command WHERE company_id=$1 AND request_id=$2::text::uuid")
            .bind(c)
            .bind(k)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
    {
        if r.get::<i64, _>("actor_id") != a || r.get::<serde_json::Value, _>("input") != *input {
            return Err(Error::Conflict);
        }
        return Ok(Some(serde_json::from_value(r.get("result")).map_err(err)?));
    }
    Ok(None)
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    k: &str,
    input: &serde_json::Value,
    d: &Document,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO inbox_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(k).bind(a).bind(input).bind(serde_json::to_value(d).map_err(err)?).execute(&mut **tx).await.map_err(err)?;
    sqlx::query("INSERT INTO inbox_audit(company_id,document_id,actor_id,revision,action) VALUES($1,$2,$3,$4,$5)").bind(c).bind(id(&d.id)?).bind(a).bind(d.revision).bind(action).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
fn file(input: &Upload) -> Result<(Vec<u8>, &'static str, String)> {
    if input.filename.is_empty()
        || input.filename.len() > 180
        || input.filename.trim() != input.filename
        || input.filename.starts_with('.')
        || input.filename.chars().any(|ch| {
            ch.is_control()
                || "/\\:\"<>|?*".contains(ch)
                || ('\u{202a}'..='\u{202e}').contains(&ch)
                || ('\u{2066}'..='\u{2069}').contains(&ch)
        })
        || input.base64.len() > 2796204
    {
        return Err(invalid());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&input.base64)
        .map_err(|_| invalid())?;
    if bytes.is_empty() || bytes.len() > 2097152 {
        return Err(invalid());
    }
    let ext = input
        .filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let media = match ext.as_str() {
        "pdf" if bytes.starts_with(b"%PDF-") => "application/pdf",
        "png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => "image/png",
        "jpg" | "jpeg" if bytes.starts_with(b"\xff\xd8\xff") => "image/jpeg",
        "isdoc" if std::str::from_utf8(&bytes).is_ok() && !bytes.contains(&0) => "application/xml",
        _ => return Err(invalid()),
    };
    let hash = format!("{:x}", Sha256::digest(&bytes));
    Ok((bytes, media, hash))
}
#[async_trait]
impl InboxStore for PgInbox {
    async fn history(&self, a: &str, c: &str, i: &str) -> Result<Vec<Audit>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        load(&mut tx, c, i).await?;
        let rows = sqlx::query(
            "SELECT * FROM inbox_audit WHERE company_id=$1 AND document_id=$2 ORDER BY id DESC",
        )
        .bind(c)
        .bind(id(i)?)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
        Ok(rows
            .into_iter()
            .map(|r| Audit {
                id: r.get::<i64, _>("id").to_string(),
                revision: r.get("revision"),
                action: r.get("action"),
                created_at: r
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
            })
            .collect())
    }

    async fn extract(
        &self,
        a: &str,
        c: &str,
        id: &str,
    ) -> Result<faktura_application::received::Data> {
        let (d, bytes) = self.download(a, c, id).await?;
        if !d.filename.to_ascii_lowercase().ends_with(".isdoc") {
            return Err(Error::Invalid(vec!["isdoc_unsupported".into()]));
        }
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&self.0)
                .await
                .map_err(err)?;
        crate::inbox_isdoc::extract(&bytes, today)
    }

    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        let page = q.page.unwrap_or(1);
        if !(1..=10000).contains(&page) || q.query.len() > 200 {
            return Err(invalid());
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let total=sqlx::query_scalar("SELECT count(*) FROM inbox_document WHERE company_id=$1 AND archived=$2 AND strpos(lower(filename),lower($3))>0").bind(c).bind(q.archived).bind(&q.query).fetch_one(&mut *tx).await.map_err(err)?;
        let rows=sqlx::query("SELECT i.*,(SELECT kind FROM received_invoice r WHERE r.company_id=i.company_id AND r.id=i.received_id) AS received_kind FROM inbox_document i WHERE i.company_id=$1 AND archived=$2 AND strpos(lower(filename),lower($3))>0 ORDER BY i.id DESC LIMIT 25 OFFSET $4").bind(c).bind(q.archived).bind(q.query).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?;
        Ok(Page {
            items: rows.iter().map(map).collect(),
            total,
            page,
        })
    }
    async fn upload(&self, a: &str, c: &str, cmd: Upload) -> Result<Document> {
        let (bytes, media, hash) = file(&cmd)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, a, c).await?;
        let input = serde_json::json!({"upload":cmd.filename,"sha256":hash});
        if let Some(d) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(d);
        }
        let existing =
            sqlx::query("SELECT * FROM inbox_document WHERE company_id=$1 AND sha256=$2")
                .bind(c)
                .bind(&hash)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        let d = if let Some(r) = existing {
            map(&r)
        } else {
            let used: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(size),0)::bigint FROM inbox_document WHERE company_id=$1",
            )
            .bind(c)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
            if used + bytes.len() as i64 > 268435456 {
                return Err(invalid());
            }
            map(&sqlx::query("INSERT INTO inbox_document(company_id,actor_id,filename,media_type,size,sha256,content) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING *").bind(c).bind(a).bind(cmd.filename).bind(media).bind(bytes.len() as i64).bind(hash).bind(bytes).fetch_one(&mut *tx).await.map_err(err)?)
        };
        record(&mut tx, a, c, &cmd.request_id, &input, &d, "upload").await?;
        tx.commit().await.map_err(err)?;
        Ok(d)
    }
    async fn download(&self, a: &str, c: &str, i: &str) -> Result<(Document, Vec<u8>)> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let r = load(&mut tx, c, i).await?;
        Ok((map(&r), r.get("content")))
    }
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Document> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, a, c).await?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(d) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(d);
        }
        let row = load(&mut tx, c, &cmd.id).await?;
        let old = map(&row);
        if old.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        let action = match cmd.action {
            Action::Archive { archived } => {
                sqlx::query("UPDATE inbox_document SET archived=$3,revision=revision+1 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&cmd.id)?).bind(archived).execute(&mut *tx).await.map_err(err)?;
                "archive"
            }
            action => {
                if old.archived || old.received_id.is_some() {
                    return Err(Error::Conflict);
                }
                let target = match action {
                    Action::Convert { data } => {
                        let create = faktura_application::received::Create {
                            request_id: uuid::Uuid::new_v4().to_string(),
                            data: *data,
                        };
                        let doc = crate::received::create_in_tx(
                            &mut tx,
                            a,
                            c,
                            &create,
                            serde_json::json!({"inbox":cmd.id,"create":create}),
                        )
                        .await?;
                        id(&doc.id)?
                    }
                    Action::Attach { received_id } => {
                        let doc = crate::received::load(&mut tx, c, id(&received_id)?).await?;
                        if doc.cancelled {
                            return Err(Error::Conflict);
                        }
                        id(&doc.id)?
                    }
                    _ => unreachable!(),
                };
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM document_attachment WHERE company_id=$1 AND received_id=$2 AND deleted_at IS NULL").bind(c).bind(target).fetch_one(&mut *tx).await.map_err(err)?;
                if count >= 5 {
                    return Err(invalid());
                }
                let used:i64=sqlx::query_scalar("SELECT coalesce(sum(octet_length(content)),0)::bigint FROM document_attachment WHERE company_id=$1").bind(c).fetch_one(&mut *tx).await.map_err(err)?;
                if used + old.size > 268435456 {
                    return Err(invalid());
                }
                sqlx::query("INSERT INTO document_attachment(company_id,received_id,request_id,actor_id,filename,media_type,size,sha256,content) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(c).bind(target).bind(uuid::Uuid::new_v4()).bind(a).bind(&old.filename).bind(&old.media_type).bind(old.size).bind(&old.sha256).bind(row.get::<Vec<u8>,_>("content")).execute(&mut *tx).await.map_err(err)?;
                sqlx::query("UPDATE inbox_document SET received_id=$3,revision=revision+1 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&cmd.id)?).bind(target).execute(&mut *tx).await.map_err(err)?;
                "process"
            }
        };
        let d = map(&load(&mut tx, c, &cmd.id).await?);
        record(&mut tx, a, c, &cmd.request_id, &input, &d, action).await?;
        tx.commit().await.map_err(err)?;
        Ok(d)
    }
}

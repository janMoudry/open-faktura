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
    async fn settings(&self, a: &str, c: &str) -> Result<Settings> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        settings_in_tx(&mut tx, c).await
    }
    async fn configure(&self, a: &str, c: &str, v: Settings) -> Result<Settings> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, a, c).await?;
        if let Some(name) = &v.extraction_provider {
            provider(name)?;
        }
        let old = settings_in_tx(&mut tx, c).await?;
        if old.revision != v.revision {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO inbox_settings(company_id,actor_id,revision,email_enabled,extraction_provider) VALUES($1,$2,1,$3,$4) ON CONFLICT(company_id) DO UPDATE SET actor_id=$2,revision=inbox_settings.revision+1,email_enabled=$3,extraction_provider=$4,updated_at=now()")
            .bind(c).bind(a).bind(v.email_enabled).bind(v.extraction_provider).execute(&mut *tx).await.map_err(err)?;
        let result = settings_in_tx(&mut tx, c).await?;
        sqlx::query("INSERT INTO inbox_settings_audit(company_id,actor_id,revision,email_enabled,extraction_provider) VALUES($1,$2,$3,$4,$5)").bind(c).bind(a).bind(result.revision).bind(result.email_enabled).bind(&result.extraction_provider).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn mail_history(&self, a: &str, c: &str) -> Result<Vec<MailDiagnostic>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let rows = sqlx::query("SELECT message_id,sender,subject,status,attachment_count,created_at FROM inbox_mail WHERE company_id=$1 ORDER BY created_at DESC,message_id DESC LIMIT 100").bind(c).fetch_all(&mut *tx).await.map_err(err)?;
        Ok(rows
            .into_iter()
            .map(|r| MailDiagnostic {
                message_id: r.get("message_id"),
                sender: r.get("sender"),
                subject: r.get("subject"),
                status: r.get("status"),
                attachment_count: r.get("attachment_count"),
                created_at: r
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
            })
            .collect())
    }
    async fn mail(&self, actor: &str, company: &str, input: MailIntake) -> Result<MailResult> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = lock(&mut tx, actor, company).await?;
        let result = mail_in_tx(&mut tx, a, c, input, false).await?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn extract_provider(
        &self,
        a: &str,
        c: &str,
        document: &str,
    ) -> Result<faktura_application::received::Data> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, company) = crate::draft::authorize(&mut tx, a, c, true).await?;
        let name = settings_in_tx(&mut tx, company)
            .await?
            .extraction_provider
            .ok_or_else(invalid)?;
        let (url, token) = provider(&name)?;
        drop(tx);
        let (document, bytes) = self.download(a, c, document).await?;
        let data = request_extraction(document, bytes, url, token).await?;
        let mut tx = self.0.begin().await.map_err(err)?;
        crate::draft::authorize(&mut tx, a, c, true).await?;
        if settings_in_tx(&mut tx, company)
            .await?
            .extraction_provider
            .as_deref()
            != Some(&name)
        {
            return Err(Error::Conflict);
        }
        Ok(data)
    }

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
        if !matches!(
            d.media_type.as_str(),
            "application/xml" | "application/pdf" | "image/png" | "image/jpeg"
        ) {
            return Err(Error::Invalid(vec!["isdoc_unsupported".into()]));
        }
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&self.0)
                .await
                .map_err(err)?;
        tokio::task::spawn_blocking(move || {
            if d.media_type.starts_with("image/") {
                crate::inbox_isdoc::extract_qr(&bytes, today)
            } else if d.media_type == "application/pdf" {
                crate::inbox_isdoc::extract_pdf(&bytes, today)
            } else {
                crate::inbox_isdoc::extract(&bytes, today)
            }
        })
        .await
        .map_err(err)?
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

async fn settings_in_tx(tx: &mut Transaction<'_, Postgres>, c: i64) -> Result<Settings> {
    let row = sqlx::query(
        "SELECT revision,email_enabled,extraction_provider FROM inbox_settings WHERE company_id=$1",
    )
    .bind(c)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?;
    Ok(row
        .map(|r| Settings {
            revision: r.get("revision"),
            email_enabled: r.get("email_enabled"),
            extraction_provider: r.get("extraction_provider"),
        })
        .unwrap_or_default())
}
fn provider(name: &str) -> Result<(String, Option<String>)> {
    if name.is_empty() || name.len() > 80 {
        return Err(invalid());
    }
    let config = std::env::var("INBOX_EXTRACTION_PROVIDERS_JSON").map_err(|_| invalid())?;
    let config: serde_json::Value = serde_json::from_str(&config).map_err(|_| invalid())?;
    let entry = config.get(name).ok_or_else(invalid)?;
    let url = entry
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(invalid)?;
    let parsed = reqwest::Url::parse(url).map_err(|_| invalid())?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err(invalid());
    }
    let token = entry
        .get("token")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    Ok((url.into(), token))
}

async fn request_extraction(
    document: Document,
    bytes: Vec<u8>,
    url: String,
    token: Option<String>,
) -> Result<faktura_application::received::Data> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(err)?;
    let mut request=client.post(url).json(&serde_json::json!({"filename":document.filename,"media_type":document.media_type,"base64":base64::engine::general_purpose::STANDARD.encode(bytes)}));
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let mut response = request.send().await.map_err(|_| Error::Unavailable)?;
    if !response.status().is_success() {
        return Err(Error::Unavailable);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
        if bytes.len() + chunk.len() > 2097152 {
            return Err(invalid());
        }
        bytes.extend_from_slice(&chunk);
    }
    let data: faktura_application::received::Data =
        serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    // A model cannot select private contacts, stock cards or acquisitions.
    if data.supplier_contact_id.is_some()
        || data.lines.iter().any(|l| l.stock.is_some())
        || data.stock_incidental_cost.is_some()
    {
        return Err(invalid());
    }
    faktura_domain::received::totals(&data).map_err(Error::Invalid)?;
    Ok(data)
}

/// Internal shared intake; native caller must authorize and validate mailbox opt-in
/// under the same company lock before setting native=true.
pub(crate) async fn mail_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    input: MailIntake,
    native: bool,
) -> Result<MailResult> {
    uuid::Uuid::parse_str(&input.request_id).map_err(|_| invalid())?;
    if input.message_id.is_empty()
        || input.message_id.len() > 998
        || input.sender.len() > 320
        || input.subject.len() > 998
        || [&input.message_id, &input.sender, &input.subject]
            .iter()
            .any(|v| v.chars().any(char::is_control))
        || input.attachments.is_empty()
        || input.attachments.len() > 10
    {
        return Err(invalid());
    }
    if !native && !settings_in_tx(tx, c).await?.email_enabled {
        return Err(Error::Forbidden);
    }
    // Validate every attachment before committing an intake. Do not trust MIME
    // claims from a gateway; upload applies the same content signatures/limits.
    let hashes = input
        .attachments
        .iter()
        .map(file)
        .collect::<Result<Vec<_>>>()?;
    if hashes.iter().map(|v| v.0.len()).sum::<usize>() > 10 * 1024 * 1024 {
        return Err(invalid());
    }
    let hash = format!("{:x}",Sha256::digest(serde_json::to_vec(&serde_json::json!({"sender":input.sender,"subject":input.subject,"attachments":input.attachments.iter().zip(&hashes).map(|(a,h)|(&a.filename,&h.2)).collect::<Vec<_>>()})).map_err(err)?));
    let previous=sqlx::query("SELECT * FROM inbox_mail WHERE company_id=$1 AND (message_id=$2 OR request_id=$3::text::uuid)").bind(c).bind(&input.message_id).bind(&input.request_id).fetch_all(&mut **tx).await.map_err(err)?;
    if previous.len() > 1 {
        return Err(Error::Conflict);
    }
    if let Some(r) = previous.into_iter().next() {
        if r.get::<String, _>("message_id") != input.message_id
            || r.get::<String, _>("input_hash") != hash
        {
            return Err(Error::Conflict);
        }
        if let Some(v) = r.get::<Option<serde_json::Value>, _>("result") {
            return serde_json::from_value(v).map_err(err);
        }
        // The advisory lock is retained across the complete intake; unfinished
        // rows only remain after a previously failed attempt.
        sqlx::query(
            "UPDATE inbox_mail SET status='processing' WHERE company_id=$1 AND message_id=$2",
        )
        .bind(c)
        .bind(&input.message_id)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    } else {
        sqlx::query("INSERT INTO inbox_mail(company_id,message_id,request_id,actor_id,input_hash,sender,subject,status,attachment_count) VALUES($1,$2,$3::text::uuid,$4,$5,$6,$7,'processing',$8)").bind(c).bind(&input.message_id).bind(&input.request_id).bind(a).bind(&hash).bind(&input.sender).bind(&input.subject).bind(input.attachments.len() as i32).execute(&mut **tx).await.map_err(err)?;
    }
    let mut documents = Vec::new();
    for (attachment, (bytes, media, hash)) in input.attachments.iter().zip(hashes) {
        let row = sqlx::query("SELECT * FROM inbox_document WHERE company_id=$1 AND sha256=$2")
            .bind(c)
            .bind(&hash)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?;
        let d = if let Some(row) = row {
            map(&row)
        } else {
            let used: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(size),0)::bigint FROM inbox_document WHERE company_id=$1",
            )
            .bind(c)
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
            if used + bytes.len() as i64 > 268435456 {
                return Err(invalid());
            }
            map(&sqlx::query("INSERT INTO inbox_document(company_id,actor_id,filename,media_type,size,sha256,content) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING *").bind(c).bind(a).bind(&attachment.filename).bind(media).bind(bytes.len() as i64).bind(hash).bind(bytes).fetch_one(&mut **tx).await.map_err(err)?)
        };
        sqlx::query("INSERT INTO inbox_audit(company_id,document_id,actor_id,revision,action) VALUES($1,$2,$3,$4,'mail')").bind(c).bind(id(&d.id)?).bind(a).bind(d.revision).execute(&mut **tx).await.map_err(err)?;
        documents.push(d);
    }
    let result = MailResult {
        message_id: input.message_id,
        documents,
    };
    sqlx::query(
        "UPDATE inbox_mail SET status='completed',result=$3 WHERE company_id=$1 AND message_id=$2",
    )
    .bind(c)
    .bind(&result.message_id)
    .bind(serde_json::to_value(&result).map_err(err)?)
    .execute(&mut **tx)
    .await
    .map_err(err)?;
    Ok(result)
}

#[cfg(test)]
mod adapter_tests {
    use super::*;
    #[tokio::test]
    async fn provider_loopback_contract_bounded_output_and_untrusted_links() {
        use std::io::{Read, Write};
        let source = crate::inbox_isdoc::extract(
            include_bytes!("../../../tests/fixtures/inbox/synthetic-basic.isdoc"),
            "2026-10-18".parse().unwrap(),
        )
        .unwrap();
        let good = serde_json::to_vec(&source).unwrap();
        let mut linked = serde_json::to_value(&source).unwrap();
        linked["supplier_contact_id"] = serde_json::json!("123");
        for (status, body, success) in [
            ("200 OK", good, true),
            ("200 OK", serde_json::to_vec(&linked).unwrap(), false),
            ("200 OK", vec![b'x'; 2097153], false),
            ("302 Found", Vec::new(), false),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/extract", listener.local_addr().unwrap());
            let thread = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut received = Vec::new();
                let mut part = [0; 4096];
                loop {
                    let n = stream.read(&mut part).unwrap();
                    if n == 0 {
                        break;
                    }
                    received.extend_from_slice(&part[..n]);
                    if let Some(end) = received.windows(4).position(|v| v == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&received[..end]);
                        let length: usize = headers
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse().unwrap())
                            })
                            .unwrap();
                        if received.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8(received).unwrap();
                assert!(request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer synthetic-only-token"));
                assert!(request.contains("\"base64\":\"JVBERg==\""));
                let header=format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nLocation: http://127.0.0.1:9/must-not-follow\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                stream.write_all(header.as_bytes()).unwrap();
                let _ = stream.write_all(&body);
            });
            let document = Document {
                id: "1".into(),
                revision: 1,
                filename: "synthetic.pdf".into(),
                media_type: "application/pdf".into(),
                size: 4,
                sha256: String::new(),
                archived: false,
                received_id: None,
                received_kind: None,
                created_at: String::new(),
            };
            let result = request_extraction(
                document,
                b"%PDF".to_vec(),
                url,
                Some("synthetic-only-token".into()),
            )
            .await;
            assert_eq!(result.is_ok(), success);
            thread.join().unwrap();
        }
    }
}

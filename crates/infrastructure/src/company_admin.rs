use async_trait::async_trait;
use faktura_application::company_admin::*;
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgAdministration(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
async fn snapshot(tx: &mut Transaction<'_, Postgres>, company: i64) -> Result<Snapshot> {
    let r =
        sqlx::query("SELECT revision,deleted_at IS NOT NULL AS deleted FROM company WHERE id=$1")
            .bind(company)
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    let members =
        sqlx::query("SELECT m.user_id,m.role,u.email,u.name FROM membership m JOIN app_user u ON u.id=m.user_id WHERE m.company_id=$1 ORDER BY m.user_id")
            .bind(company)
            .fetch_all(&mut **tx)
            .await
            .map_err(err)?
            .into_iter()
            .map(|r| Member {
                user_id: r.get::<i64, _>("user_id").to_string(),
                role: r.get("role"),
            email: r.get("email"),
            name: r.get("name"),
            })
            .collect();
    Ok(Snapshot {
        company_id: company.to_string(),
        revision: r.get("revision"),
        deleted: r.get("deleted"),
        members,
    })
}
#[async_trait]
impl AdministrationStore for PgAdministration {
    async fn list(&self, actor: &str, company: &str) -> Result<Snapshot> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let company = id(company)?;
        let actor = id(actor)?;
        let role:Option<String>=sqlx::query_scalar("SELECT m.role FROM membership m JOIN company c ON c.id=m.company_id WHERE c.id=$1 AND m.user_id=$2 AND (c.deleted_at IS NULL OR m.role='owner') FOR SHARE OF c,m").bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(err)?;
        if role.is_none() {
            return Err(Error::NotFound);
        }
        let value = snapshot(&mut tx, company).await?;
        tx.commit().await.map_err(err)?;
        Ok(value)
    }
    async fn trash(&self, actor: &str) -> Result<Vec<Snapshot>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let ids:Vec<i64>=sqlx::query_scalar("SELECT c.id FROM company c JOIN membership m ON m.company_id=c.id WHERE m.user_id=$1 AND m.role='owner' AND c.deleted_at IS NOT NULL ORDER BY c.id").bind(id(actor)?).fetch_all(&mut *tx).await.map_err(err)?;
        let mut values = Vec::new();
        for company in ids {
            values.push(snapshot(&mut tx, company).await?)
        }
        tx.commit().await.map_err(err)?;
        Ok(values)
    }
    async fn execute(&self, actor: &str, company: &str, command: Command) -> Result<Snapshot> {
        let actor = id(actor)?;
        let company = id(company)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        // All administration changes share the existing company write lock. It conflicts
        // with document authorization FOR SHARE and company update/delete.
        let row=sqlx::query("SELECT c.revision,c.settings,c.deleted_at IS NOT NULL AS deleted,m.role FROM company c LEFT JOIN membership m ON m.company_id=c.id AND m.user_id=$2 WHERE c.id=$1 FOR UPDATE OF c").bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        let role: String = row
            .get::<Option<String>, _>("role")
            .ok_or(Error::NotFound)?;
        if uuid::Uuid::parse_str(&command.request_id).is_err() || command.revision < 1 {
            return Err(Error::Invalid(vec!["request_id".into(), "revision".into()]));
        }
        let input = serde_json::to_value(&command).map_err(err)?;
        if let Some(old)=sqlx::query("SELECT actor_id,input,result FROM company_admin_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(company).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)?{
   if old.get::<i64,_>("actor_id")!=actor || old.get::<serde_json::Value,_>("input")!=input{return Err(Error::Conflict)}
   return serde_json::from_value(old.get("result")).map_err(err)
  }
        if !matches!(command.action, Action::Leave) && role != "owner" {
            return Err(Error::Forbidden);
        }
        if row.get::<i64, _>("revision") != command.revision {
            return Err(Error::Conflict);
        }
        if row.get::<bool, _>("deleted") != matches!(command.action, Action::Restore) {
            return Err(Error::Conflict);
        }
        let action = match &command.action {
            Action::Restore => {
                sqlx::query("UPDATE company SET deleted_at=NULL WHERE id=$1")
                    .bind(company)
                    .execute(&mut *tx)
                    .await
                    .map_err(err)?;
                "restore"
            }
            Action::Leave => {
                if role == "owner" {
                    return Err(Error::Conflict);
                }
                sqlx::query("DELETE FROM membership WHERE company_id=$1 AND user_id=$2")
                    .bind(company)
                    .bind(actor)
                    .execute(&mut *tx)
                    .await
                    .map_err(err)?;
                "leave"
            }
            Action::AddMember { email, role } => {
                if email.len() > 254 || !email.contains('@') {
                    return Err(Error::Invalid(vec!["email".into()]));
                }
                let user: i64 = sqlx::query_scalar("SELECT id FROM app_user WHERE email=$1")
                    .bind(email.trim().to_lowercase())
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(err)?
                    .ok_or(Error::NotFound)?;
                let count=sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(company).bind(user).bind(role.as_str()).execute(&mut *tx).await.map_err(err)?.rows_affected();
                if count != 1 {
                    return Err(Error::Conflict);
                }
                "member_add"
            }
            Action::SetRole { user_id, role } => {
                let count=sqlx::query("UPDATE membership SET role=$3 WHERE company_id=$1 AND user_id=$2 AND role<>'owner'").bind(company).bind(id(user_id)?).bind(role.as_str()).execute(&mut *tx).await.map_err(err)?.rows_affected();
                if count != 1 {
                    return Err(Error::Conflict);
                }
                "member_role"
            }
            Action::RemoveMember { user_id } => {
                let count = sqlx::query(
                    "DELETE FROM membership WHERE company_id=$1 AND user_id=$2 AND role<>'owner'",
                )
                .bind(company)
                .bind(id(user_id)?)
                .execute(&mut *tx)
                .await
                .map_err(err)?
                .rows_affected();
                if count != 1 {
                    return Err(Error::Conflict);
                }
                "member_remove"
            }
            Action::Transfer { user_id } => {
                let target = id(user_id)?;
                if target == actor {
                    return Err(Error::Conflict);
                }
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM membership WHERE company_id=$1 AND user_id=$2)",
                )
                .bind(company)
                .bind(target)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
                if !exists {
                    return Err(Error::NotFound);
                }
                sqlx::query(
                    "UPDATE membership SET role='editor' WHERE company_id=$1 AND user_id=$2",
                )
                .bind(company)
                .bind(actor)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
                sqlx::query(
                    "UPDATE membership SET role='owner' WHERE company_id=$1 AND user_id=$2",
                )
                .bind(company)
                .bind(target)
                .execute(&mut *tx)
                .await
                .map_err(err)?;
                "transfer"
            }
        };
        sqlx::query("UPDATE company SET revision=revision+1,updated_at=now() WHERE id=$1")
            .bind(company)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        sqlx::query("INSERT INTO company_audit(company_id,revision,actor_id,action,settings) VALUES($1,$2,$3,$4,$5)").bind(company).bind(command.revision+1).bind(actor).bind(action).bind(row.get::<serde_json::Value,_>("settings")).execute(&mut *tx).await.map_err(err)?;
        let result = snapshot(&mut tx, company).await?;
        sqlx::query("INSERT INTO company_admin_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(company).bind(&command.request_id).bind(actor).bind(input).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

#[async_trait]
impl ArchiveStore for PgAdministration {
    async fn export(&self, actor: &str, company: &str) -> Result<Archive> {
        use std::io::{Cursor, Write};
        static ARCHIVES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
        let _permit = ARCHIVES.acquire().await.map_err(err)?;
        let actor = id(actor)?;
        let company = id(company)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let role:Option<String>=sqlx::query_scalar("SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR SHARE OF c,m").bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(err)?;
        if role.as_deref() != Some("owner") {
            return Err(if role.is_none() {
                Error::NotFound
            } else {
                Error::Forbidden
            });
        }
        // Explicit reviewed table AND column allowlist. New schema fields are opt-in.
        let tables: std::collections::BTreeMap<String, Vec<String>> = serde_json::from_str(
            include_str!("../../../contracts/company-archive-schema.json"),
        )
        .map_err(err)?;
        let links:Vec<String>=sqlx::query_scalar("SELECT url FROM quote_share WHERE company_id=$1 UNION SELECT url FROM invoice_share WHERE company_id=$1").bind(company).fetch_all(&mut *tx).await.map_err(err)?;
        let mut secrets = Vec::new();
        for link in links {
            if let Some(token) = link.rsplit('/').next() {
                if token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()) {
                    secrets.push(token.to_owned());
                }
            }
            secrets.push(link);
        }
        secrets.sort_by_key(|v| std::cmp::Reverse(v.len()));
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let mut manifest = Vec::new();
        let mut total_bytes = 0usize;
        let mut total_rows = 0usize;
        for (table, columns) in &tables {
            if !table
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                return Err(Error::Unavailable);
            }
            let key = if table == "company" {
                "id"
            } else {
                "company_id"
            };
            let mut offset = 0i64;
            let mut count = 0usize;
            zip.start_file(format!("tables/{table}.jsonl"), options)
                .map_err(err)?;
            loop {
                let selected = columns
                    .iter()
                    .map(|column| format!("\"{}\"", column))
                    .collect::<Vec<_>>()
                    .join(",");
                let query=format!("SELECT to_jsonb(projected)::text FROM (SELECT {selected} FROM public.\"{table}\" WHERE {key}=$1 ORDER BY ctid LIMIT 100 OFFSET $2) projected");
                let rows: Vec<String> = sqlx::query_scalar(&query)
                    .bind(company)
                    .bind(offset)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(err)?;
                let len = rows.len();
                for row in rows {
                    let mut value: serde_json::Value = serde_json::from_str(&row).map_err(err)?;
                    redact_links(&mut value, &secrets);
                    let row = serde_json::to_string(&value).map_err(err)?;
                    total_bytes += row.len() + 1;
                    total_rows += 1;
                    count += 1;
                    if total_bytes > 100 * 1024 * 1024 || total_rows > 100_000 {
                        return Err(Error::Invalid(vec!["archive_limit".into()]));
                    }
                    zip.write_all(row.as_bytes()).map_err(err)?;
                    zip.write_all(b"\n").map_err(err)?;
                }
                if len < 100 {
                    break;
                }
                offset += 100;
            }
            manifest.push(serde_json::json!({"table":table,"rows":count}));
        }
        let schema_version: Option<i64> =
            sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        let value = serde_json::json!({"format":"open-faktura-company-archive","version":1,"schema_version":schema_version,"company_id":company.to_string(),"tables":manifest,"allowed_columns":tables,"share_links_redacted":true,"excluded_columns":["secret","password","token_hash","url"],"binary_encoding":"PostgreSQL bytea hex","limits":{"rows":100000,"uncompressed_bytes":104857600},"restore_backup":false});
        zip.start_file("manifest.json", options).map_err(err)?;
        zip.write_all(&serde_json::to_vec_pretty(&value).map_err(err)?)
            .map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(Archive {
            bytes: zip.finish().map_err(err)?.into_inner(),
            file_name: format!("company-{company}-archive.zip"),
        })
    }
}

fn redact_links(value: &mut serde_json::Value, secrets: &[String]) {
    match value {
        serde_json::Value::String(text) => {
            for secret in secrets {
                *text = text.replace(secret, "[redacted-public-link]");
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_links(item, secrets)
            }
        }
        serde_json::Value::Object(items) => {
            for item in items.values_mut() {
                redact_links(item, secrets)
            }
        }
        _ => {}
    }
}

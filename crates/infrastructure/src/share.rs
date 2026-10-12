use crate::draft::authorize;
use async_trait::async_trait;
use faktura_application::{
    sales::{Document, Kind},
    share::*,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgShare {
    pub pool: PgPool,
    pub base_url: Option<String>,
}
pub fn base_url_from_env() -> std::result::Result<Option<String>, String> {
    let Ok(raw) = std::env::var("PUBLIC_WEB_URL") else {
        return Ok(None);
    };
    let url = reqwest::Url::parse(&raw).map_err(|_| "Invalid PUBLIC_WEB_URL")?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))))
    {
        return Err("PUBLIC_WEB_URL must be HTTPS (HTTP only for loopback), without credentials/query/fragment".into());
    }
    Ok(Some(url.to_string().trim_end_matches('/').into()))
}
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(s: &str) -> Result<i64> {
    s.parse().map_err(|_| Error::NotFound)
}
fn hash(t: &str) -> String {
    format!("{:x}", Sha256::digest(t.as_bytes()))
}
fn timestamp(r: &sqlx::postgres::PgRow, k: &str) -> Option<String> {
    r.get::<Option<chrono::DateTime<chrono::Utc>>, _>(k)
        .map(|v| v.to_rfc3339())
}
fn map(r: sqlx::postgres::PgRow) -> Link {
    Link {
        id: r.get::<i64, _>("id").to_string(),
        revision: r.get("revision"),
        url: r.get("url"),
        created_at: timestamp(&r, "created_at").unwrap_or_default(),
        expires_at: timestamp(&r, "expires_at").unwrap_or_default(),
        revoked_at: timestamp(&r, "revoked_at"),
        view_count: r.get("view_count"),
        first_viewed_at: timestamp(&r, "first_viewed_at"),
        last_viewed_at: timestamp(&r, "last_viewed_at"),
    }
}
const STATS:&str="SELECT s.*, (SELECT count(*) FROM quote_view v WHERE v.share_id=s.id) AS view_count,(SELECT min(viewed_at) FROM quote_view v WHERE v.share_id=s.id) AS first_viewed_at,(SELECT max(viewed_at) FROM quote_view v WHERE v.share_id=s.id) AS last_viewed_at FROM quote_share s";
async fn quote(tx: &mut Transaction<'_, Postgres>, c: i64, d: i64) -> Result<Document> {
    crate::sales::map(sqlx::query("SELECT s.*,i.number AS linked_number FROM sales_document s LEFT JOIN invoice_draft i ON i.company_id=s.company_id AND i.id=s.invoice_id WHERE s.company_id=$1 AND s.id=$2 AND s.kind='quote' FOR SHARE OF s").bind(c).bind(d).fetch_optional(&mut **tx).await.map_err(err)?.ok_or(Error::NotFound)?)
}
impl PgShare {
    async fn resolve(&self, t: &str) -> Result<Document> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let r=sqlx::query("SELECT l.company_id,l.document_id FROM quote_share l JOIN company c ON c.id=l.company_id WHERE l.token_hash=$1 AND l.revoked_at IS NULL AND l.expires_at>now() AND c.deleted_at IS NULL FOR SHARE OF l,c").bind(hash(t)).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        let mut d = quote(&mut tx, r.get("company_id"), r.get("document_id")).await?;
        if d.archived {
            return Err(Error::NotFound);
        }
        // Public DTO and PDF intentionally exclude internal contact linkage and free-form internal note.
        d.data.invoice.contact_id = None;
        d.data.invoice.note.clear();
        tx.commit().await.map_err(err)?;
        Ok(d)
    }
}
#[async_trait]
impl ShareStore for PgShare {
    async fn create(&self, a: &str, c: &str, d: &str, cmd: Create) -> Result<Link> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        let d = id(d)?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        if let Some(r) = sqlx::query(&format!(
            "{STATS} WHERE s.company_id=$1 AND s.request_id=$2::text::uuid"
        ))
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != a
                || r.get::<i64, _>("document_id") != d
                || r.get::<i64, _>("revision") != cmd.revision
                || r.get::<i32, _>("expires_days") != cmd.expires_days
            {
                return Err(Error::Conflict);
            }
            return Ok(map(r));
        }
        let q = quote(&mut tx, c, d).await?;
        if q.revision != cmd.revision || q.archived {
            return Err(Error::Conflict);
        }
        let base = self
            .base_url
            .as_ref()
            .ok_or_else(|| Error::Invalid(vec!["public_web_url".into()]))?;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        // URL is confidential and retained for authorized re-copy/idempotent replay, just like sent email bodies.
        let url = format!("{base}/#/shared/quote/{token}");
        let sid:i64=sqlx::query_scalar("INSERT INTO quote_share(company_id,document_id,actor_id,request_id,revision,expires_days,token_hash,url,expires_at) VALUES($1,$2,$3,$4::text::uuid,$5,$6,$7,$8,now()+make_interval(days=>$6)) RETURNING id").bind(c).bind(d).bind(a).bind(cmd.request_id).bind(cmd.revision).bind(cmd.expires_days).bind(hash(&token)).bind(url).fetch_one(&mut *tx).await.map_err(err)?;
        let result = map(sqlx::query(&format!("{STATS} WHERE s.id=$1"))
            .bind(sid)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?);
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn list(&self, a: &str, c: &str, d: &str, page: i64) -> Result<Links> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let d = id(d)?;
        quote(&mut tx, c, d).await?;
        let total = sqlx::query_scalar(
            "SELECT count(*) FROM quote_share WHERE company_id=$1 AND document_id=$2",
        )
        .bind(c)
        .bind(d)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items=sqlx::query(&format!("{STATS} WHERE s.company_id=$1 AND s.document_id=$2 ORDER BY s.id DESC LIMIT 25 OFFSET $3")).bind(c).bind(d).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(map).collect();
        Ok(Links {
            configured: self.base_url.is_some(),
            items,
            total,
            page,
        })
    }
    async fn revoke(&self, a: &str, c: &str, d: &str, s: &str) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        let r=sqlx::query("UPDATE quote_share SET revoked_at=COALESCE(revoked_at,now()),revoked_by=COALESCE(revoked_by,$4) WHERE company_id=$1 AND document_id=$2 AND id=$3").bind(c).bind(id(d)?).bind(id(s)?).bind(a).execute(&mut *tx).await.map_err(err)?;
        if r.rows_affected() == 0 {
            return Err(Error::NotFound);
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
    async fn views(&self, a: &str, c: &str, d: &str, s: &str, page: i64) -> Result<Views> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM quote_share WHERE company_id=$1 AND document_id=$2 AND id=$3)").bind(c).bind(id(d)?).bind(id(s)?).fetch_one(&mut *tx).await.map_err(err)?;
        if !exists {
            return Err(Error::NotFound);
        }
        let total = sqlx::query_scalar("SELECT count(*) FROM quote_view WHERE share_id=$1")
            .bind(id(s)?)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        let items=sqlx::query("SELECT revision,viewed_at FROM quote_view WHERE share_id=$1 ORDER BY id DESC LIMIT 25 OFFSET $2").bind(id(s)?).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(|r|View{revision:r.get("revision"),viewed_at:timestamp(&r,"viewed_at").unwrap_or_default()}).collect();
        Ok(Views { items, page, total })
    }
    async fn public_quote(&self, t: &str) -> Result<PublicQuote> {
        let q = self.resolve(t).await?;
        Ok(PublicQuote {
            number: q.number.unwrap_or_default(),
            revision: q.revision,
            name: q.data.name,
            status: q.data.status,
            invoice: q.data.invoice,
            totals: q.totals.ok_or(Error::Unavailable)?,
        })
    }
    async fn public_pdf(&self, t: &str) -> Result<Vec<u8>> {
        let q = self.resolve(t).await?;
        let bytes = crate::delivery::render(q).await?;
        self.resolve(t).await?;
        Ok(bytes)
    }
    async fn view(&self, t: &str, visit: &str, revision: i64) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let r=sqlx::query("SELECT l.id,l.company_id,l.document_id FROM quote_share l JOIN company c ON c.id=l.company_id WHERE l.token_hash=$1 AND l.revoked_at IS NULL AND l.expires_at>now() AND c.deleted_at IS NULL FOR UPDATE OF l").bind(hash(t)).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        let q = quote(&mut tx, r.get("company_id"), r.get("document_id")).await?;
        if q.kind != Kind::Quote || q.archived {
            return Err(Error::NotFound);
        }
        if revision > q.revision {
            return Err(Error::Invalid(vec!["revision".into()]));
        }
        // Cap persistent event growth without IP addresses, cookies, or fingerprinting.
        let recent:i64=sqlx::query_scalar("SELECT count(*) FROM quote_view WHERE share_id=$1 AND viewed_at>now()-interval '1 hour'").bind(r.get::<i64,_>("id")).fetch_one(&mut *tx).await.map_err(err)?;
        if recent < 60 {
            sqlx::query("INSERT INTO quote_view(company_id,document_id,share_id,visit_id,revision) VALUES($1,$2,$3,$4::text::uuid,$5) ON CONFLICT(share_id,visit_id) DO NOTHING").bind(q.company_id.parse::<i64>().map_err(err)?).bind(q.id.parse::<i64>().map_err(err)?).bind(r.get::<i64,_>("id")).bind(visit).bind(revision).execute(&mut *tx).await.map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
}

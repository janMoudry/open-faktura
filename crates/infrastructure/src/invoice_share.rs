use crate::draft::authorize;
use async_trait::async_trait;
use faktura_application::{
    draft::{Draft as Document, DraftStore},
    share::*,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgInvoiceShare {
    pub pool: PgPool,
    pub base_url: Option<String>,
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
const STATS:&str="SELECT s.*, (SELECT count(*) FROM invoice_view v WHERE v.share_id=s.id) AS view_count,(SELECT min(viewed_at) FROM invoice_view v WHERE v.share_id=s.id) AS first_viewed_at,(SELECT max(viewed_at) FROM invoice_view v WHERE v.share_id=s.id) AS last_viewed_at FROM invoice_share s";
async fn invoice(tx: &mut Transaction<'_, Postgres>, c: i64, d: i64) -> Result<Document> {
    let value = crate::draft::map(
        sqlx::query("SELECT * FROM invoice_with_settlement WHERE company_id=$1 AND id=$2")
            .bind(c)
            .bind(d)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?,
    )?;
    if value.number.is_none() || value.data.calculation.calculation_version == "import-v1" {
        return Err(Error::Conflict);
    }
    Ok(value)
}
impl PgInvoiceShare {
    async fn resolve(&self, t: &str) -> Result<Document> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let r=sqlx::query("SELECT l.company_id,l.document_id FROM invoice_share l JOIN company c ON c.id=l.company_id WHERE l.token_hash=$1 AND l.revoked_at IS NULL AND l.expires_at>now() AND c.deleted_at IS NULL FOR SHARE OF l,c").bind(hash(t)).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        let mut d = invoice(&mut tx, r.get("company_id"), r.get("document_id")).await?;
        // Public metadata excludes internal linkage. PDF is the immutable issued document.
        // The printable invoice note is part of that document; payment notes never leave the private API.
        d.data.contact_id = None;
        d.data.note.clear();
        tx.commit().await.map_err(err)?;
        Ok(d)
    }
}
#[async_trait]
impl ShareStore for PgInvoiceShare {
    async fn create(&self, a: &str, c: &str, d: &str, cmd: Create) -> Result<Link> {
        let _pdf = crate::draft::PgDraft(self.pool.clone())
            .pdf(a, c, d)
            .await?;
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
        let q = invoice(&mut tx, c, d).await?;
        if q.revision != cmd.revision {
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
        let url = format!("{base}/#/shared/invoice/{token}");
        let sid:i64=sqlx::query_scalar("INSERT INTO invoice_share(company_id,document_id,actor_id,request_id,revision,expires_days,token_hash,url,expires_at) VALUES($1,$2,$3,$4::text::uuid,$5,$6,$7,$8,now()+make_interval(days=>$6)) RETURNING id").bind(c).bind(d).bind(a).bind(cmd.request_id).bind(cmd.revision).bind(cmd.expires_days).bind(hash(&token)).bind(url).fetch_one(&mut *tx).await.map_err(err)?;
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
        invoice(&mut tx, c, d).await?;
        let total = sqlx::query_scalar(
            "SELECT count(*) FROM invoice_share WHERE company_id=$1 AND document_id=$2",
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
        let r=sqlx::query("UPDATE invoice_share SET revoked_at=COALESCE(revoked_at,now()),revoked_by=COALESCE(revoked_by,$4) WHERE company_id=$1 AND document_id=$2 AND id=$3").bind(c).bind(id(d)?).bind(id(s)?).bind(a).execute(&mut *tx).await.map_err(err)?;
        if r.rows_affected() == 0 {
            return Err(Error::NotFound);
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
    async fn views(&self, a: &str, c: &str, d: &str, s: &str, page: i64) -> Result<Views> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invoice_share WHERE company_id=$1 AND document_id=$2 AND id=$3)").bind(c).bind(id(d)?).bind(id(s)?).fetch_one(&mut *tx).await.map_err(err)?;
        if !exists {
            return Err(Error::NotFound);
        }
        let total = sqlx::query_scalar("SELECT count(*) FROM invoice_view WHERE share_id=$1")
            .bind(id(s)?)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        let items=sqlx::query("SELECT revision,viewed_at FROM invoice_view WHERE share_id=$1 ORDER BY id DESC LIMIT 25 OFFSET $2").bind(id(s)?).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(|r|View{revision:r.get("revision"),viewed_at:timestamp(&r,"viewed_at").unwrap_or_default()}).collect();
        Ok(Views { items, page, total })
    }
    async fn public_quote(&self, t: &str) -> Result<PublicQuote> {
        let q = self.resolve(t).await?;
        Ok(PublicQuote {
            number: q.number.unwrap_or_default(),
            revision: q.revision,
            name: q
                .data
                .calculation
                .lines
                .first()
                .map(|v| v.description.clone())
                .unwrap_or_default(),
            status: q
                .settlement
                .as_ref()
                .map(|v| v.status.clone())
                .unwrap_or_else(|| "unpaid".into()),
            invoice: q.data,
            totals: q.totals.ok_or(Error::Unavailable)?,
        })
    }
    async fn public_pdf(&self, t: &str) -> Result<Vec<u8>> {
        let q = self.resolve(t).await?;
        let bytes: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT pdf FROM invoice_draft WHERE company_id=$1 AND id=$2")
                .bind(id(&q.company_id)?)
                .bind(id(&q.id)?)
                .fetch_one(&self.pool)
                .await
                .map_err(err)?;
        let bytes = bytes.ok_or(Error::Unavailable)?;
        self.resolve(t).await?;
        Ok(bytes)
    }
    async fn view(&self, t: &str, visit: &str, revision: i64) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let r=sqlx::query("SELECT l.id,l.company_id,l.document_id FROM invoice_share l JOIN company c ON c.id=l.company_id WHERE l.token_hash=$1 AND l.revoked_at IS NULL AND l.expires_at>now() AND c.deleted_at IS NULL FOR UPDATE OF l").bind(hash(t)).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        let q = invoice(&mut tx, r.get("company_id"), r.get("document_id")).await?;
        if revision != q.revision {
            return Err(Error::Invalid(vec!["revision".into()]));
        }
        // Cap persistent event growth without IP addresses, cookies, or fingerprinting.
        let recent:i64=sqlx::query_scalar("SELECT count(*) FROM invoice_view WHERE share_id=$1 AND viewed_at>now()-interval '1 hour'").bind(r.get::<i64,_>("id")).fetch_one(&mut *tx).await.map_err(err)?;
        if recent < 60 {
            sqlx::query("INSERT INTO invoice_view(company_id,document_id,share_id,visit_id,revision) VALUES($1,$2,$3,$4::text::uuid,$5) ON CONFLICT(share_id,visit_id) DO NOTHING").bind(q.company_id.parse::<i64>().map_err(err)?).bind(q.id.parse::<i64>().map_err(err)?).bind(r.get::<i64,_>("id")).bind(visit).bind(revision).execute(&mut *tx).await.map_err(err)?;
        }
        tx.commit().await.map_err(err)?;
        Ok(())
    }
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub revision: i64,
    pub expires_days: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub revision: i64,
    pub url: String,
    pub created_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
    pub view_count: i64,
    pub first_viewed_at: Option<String>,
    pub last_viewed_at: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    pub configured: bool,
    pub items: Vec<Link>,
    pub page: i64,
    pub total: i64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct View {
    pub revision: i64,
    pub viewed_at: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Views {
    pub items: Vec<View>,
    pub page: i64,
    pub total: i64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicQuote {
    pub number: String,
    pub revision: i64,
    pub name: String,
    pub status: String,
    pub invoice: faktura_domain::invoice::DraftData,
    pub totals: faktura_domain::invoice::Totals,
}
#[async_trait]
pub trait ShareStore: Send + Sync {
    async fn create(&self, a: &str, c: &str, d: &str, cmd: Create) -> Result<Link>;
    async fn list(&self, a: &str, c: &str, d: &str, page: i64) -> Result<Links>;
    async fn revoke(&self, a: &str, c: &str, d: &str, id: &str) -> Result<()>;
    async fn views(&self, a: &str, c: &str, d: &str, id: &str, page: i64) -> Result<Views>;
    async fn public_quote(&self, token: &str) -> Result<PublicQuote>;
    async fn public_pdf(&self, token: &str) -> Result<Vec<u8>>;
    async fn view(&self, token: &str, visit: &str, revision: i64) -> Result<()>;
}
pub async fn create(s: &dyn ShareStore, a: &str, c: &str, d: &str, cmd: Create) -> Result<Link> {
    if uuid::Uuid::parse_str(&cmd.request_id).is_err()
        || cmd.revision < 1
        || !(1..=365).contains(&cmd.expires_days)
    {
        return Err(Error::Invalid(vec!["share".into()]));
    }
    s.create(a, c, d, cmd).await
}
pub async fn list(s: &dyn ShareStore, a: &str, c: &str, d: &str, page: i64) -> Result<Links> {
    page_check(page)?;
    s.list(a, c, d, page).await
}
pub async fn revoke(s: &dyn ShareStore, a: &str, c: &str, d: &str, id: &str) -> Result<()> {
    s.revoke(a, c, d, id).await
}
pub async fn views(
    s: &dyn ShareStore,
    a: &str,
    c: &str,
    d: &str,
    id: &str,
    page: i64,
) -> Result<Views> {
    page_check(page)?;
    s.views(a, c, d, id, page).await
}
fn page_check(p: i64) -> Result<()> {
    if !(1..=10000).contains(&p) {
        return Err(Error::Invalid(vec!["page".into()]));
    }
    Ok(())
}
fn token_check(t: &str) -> Result<()> {
    if t.len() != 64
        || !t
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(Error::NotFound);
    }
    Ok(())
}
pub async fn public_quote(s: &dyn ShareStore, t: &str) -> Result<PublicQuote> {
    token_check(t)?;
    s.public_quote(t).await
}
pub async fn public_pdf(s: &dyn ShareStore, t: &str) -> Result<Vec<u8>> {
    token_check(t)?;
    s.public_pdf(t).await
}
pub async fn view(s: &dyn ShareStore, t: &str, v: &str, r: i64) -> Result<()> {
    token_check(t)?;
    if uuid::Uuid::parse_str(v).is_err() || r < 1 {
        return Err(Error::Invalid(vec!["view".into()]));
    }
    s.view(t, v, r).await
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    pub media_type: String,
    pub size: i64,
    pub sha256: String,
    pub created_at: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upload {
    pub request_id: String,
    pub filename: String,
    pub base64: String,
}
pub struct File {
    pub metadata: Attachment,
    pub content: Vec<u8>,
}
#[async_trait]
pub trait AttachmentStore: Send + Sync {
    async fn list(&self, a: &str, c: &str, kind: &str, d: &str) -> Result<Vec<Attachment>>;
    async fn upload(
        &self,
        a: &str,
        c: &str,
        kind: &str,
        d: &str,
        cmd: Upload,
    ) -> Result<Attachment>;
    async fn download(
        &self,
        a: &str,
        c: &str,
        kind: &str,
        d: &str,
        id: &str,
        delivery: Option<&str>,
    ) -> Result<File>;
    async fn delete(&self, a: &str, c: &str, kind: &str, d: &str, id: &str) -> Result<()>;
}
pub async fn upload(
    s: &dyn AttachmentStore,
    a: &str,
    c: &str,
    k: &str,
    d: &str,
    cmd: Upload,
) -> Result<Attachment> {
    if uuid::Uuid::parse_str(&cmd.request_id).is_err()
        || cmd.filename.trim() != cmd.filename
        || cmd.filename.is_empty()
        || cmd.filename.len() > 180
        || cmd.filename.starts_with('.')
        || cmd
            .filename
            .chars()
            .any(|ch| ch.is_control() || "/\\:\"<>|?*".contains(ch))
        || cmd.filename.chars().any(|ch| {
            ('\u{202a}'..='\u{202e}').contains(&ch) || ('\u{2066}'..='\u{2069}').contains(&ch)
        })
        || cmd.base64.len() > 2796204
    {
        return Err(Error::Invalid(vec!["file".into()]));
    }
    s.upload(a, c, k, d, cmd).await
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendQuote {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<String>,
    pub request_id: String,
    pub revision: i64,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub share_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delivery {
    #[serde(default)]
    pub attachments: Vec<crate::attachment::Attachment>,
    pub id: String,
    pub document_id: String,
    pub revision: i64,
    pub actor_id: String,
    pub sender: String,
    pub reply_to: String,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub error_code: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub share_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub configured: bool,
    pub sender: Option<String>,
    pub items: Vec<Delivery>,
    pub page: i64,
    pub total: i64,
}
#[async_trait]
pub trait DeliveryStore: Send + Sync {
    async fn pdf(&self, a: &str, c: &str, id: &str, delivery: Option<&str>) -> Result<Vec<u8>>;
    async fn send(&self, a: &str, c: &str, id: &str, cmd: SendQuote) -> Result<Delivery>;
    async fn history(&self, a: &str, c: &str, id: &str, page: i64) -> Result<History>;
}
pub async fn pdf(
    s: &dyn DeliveryStore,
    a: &str,
    c: &str,
    id: &str,
    d: Option<&str>,
) -> Result<Vec<u8>> {
    s.pdf(a, c, id, d).await
}
pub async fn history(
    s: &dyn DeliveryStore,
    a: &str,
    c: &str,
    id: &str,
    page: i64,
) -> Result<History> {
    if !(1..=10000).contains(&page) {
        return Err(Error::Invalid(vec!["page".into()]));
    }
    s.history(a, c, id, page).await
}
pub async fn send(
    s: &dyn DeliveryStore,
    a: &str,
    c: &str,
    id: &str,
    mut cmd: SendQuote,
) -> Result<Delivery> {
    cmd.recipient = cmd.recipient.trim().into();
    cmd.subject = cmd.subject.trim().into();
    let mut errors = Vec::new();
    if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
        errors.push("request_id".into());
    }
    if cmd.revision < 1 {
        errors.push("revision".into());
    }
    if cmd.recipient.is_empty()
        || cmd.recipient.len() > 254
        || cmd.recipient.chars().any(char::is_control)
    {
        errors.push("recipient".into());
    }
    if cmd.subject.is_empty()
        || cmd.subject.chars().count() > 200
        || cmd.subject.chars().any(char::is_control)
    {
        errors.push("subject".into());
    }
    if cmd.body.trim().is_empty()
        || cmd.body.len() > 20000
        || cmd
            .body
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        errors.push("body".into());
    }
    cmd.attachment_ids.sort();
    if cmd.attachment_ids.len() > 5
        || cmd.attachment_ids.iter().any(|v| v.parse::<i64>().is_err())
        || cmd.attachment_ids.windows(2).any(|w| w[0] == w[1])
    {
        errors.push("attachments".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    s.send(a, c, id, cmd).await
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Send {
    pub request_id: String,
    pub revision: i64,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub confirm_original: bool,
}
impl Send {
    pub fn validate(&self) -> Result<()> {
        let mut fields = vec![];
        if uuid::Uuid::parse_str(&self.request_id).is_err() {
            fields.push("request_id".into())
        }
        if self.revision < 1 {
            fields.push("revision".into())
        }
        if !self.confirm_original {
            fields.push("confirm_original".into())
        }
        if self.recipient.is_empty()
            || self.recipient.len() > 254
            || self.recipient.trim() != self.recipient
            || self.recipient.chars().any(char::is_control)
        {
            fields.push("recipient".into())
        }
        if self.subject.trim().is_empty()
            || self.subject.chars().count() > 200
            || self.subject.chars().any(char::is_control)
        {
            fields.push("subject".into())
        }
        if self.body.trim().is_empty()
            || self.body.len() > 20000
            || self
                .body
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            fields.push("body".into())
        }
        if fields.is_empty() {
            Ok(())
        } else {
            Err(Error::Invalid(fields))
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Delivery {
    pub id: String,
    pub document_id: String,
    pub revision: i64,
    pub recipient: String,
    pub subject: String,
    pub body: String,
    pub filename: String,
    pub sha256: String,
    pub size: i64,
    pub sender: String,
    pub status: String,
    pub error_code: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct History {
    pub configured: bool,
    pub sender: Option<String>,
    pub items: Vec<Delivery>,
}
#[async_trait]
pub trait InboxDeliveryStore: std::marker::Send + Sync {
    async fn send(&self, a: &str, c: &str, id: &str, v: Send) -> Result<Delivery>;
    async fn history(&self, a: &str, c: &str, id: &str) -> Result<History>;
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub revision: i64,
    pub new_documents: bool,
    pub errors: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configure {
    pub revision: i64,
    pub new_documents: bool,
    pub errors: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub category: String,
    pub document_id: Option<String>,
    pub filename: Option<String>,
    pub code: Option<String>,
    pub created_at: String,
    pub unread: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feed {
    pub settings: Settings,
    pub events: Vec<Event>,
    pub unread: i64,
    pub through: String,
}
#[async_trait]
pub trait NotificationsStore: Send + Sync {
    async fn feed(&self, a: &str, c: &str) -> Result<Feed>;
    async fn configure(&self, a: &str, c: &str, v: Configure) -> Result<Feed>;
    async fn read(&self, a: &str, c: &str, through: &str) -> Result<Feed>;
}

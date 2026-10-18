pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Trash,
    Restore,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub request_id: String,
    pub id: String,
    pub revision: i64,
    pub action: Action,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub customer: String,
    pub revision: i64,
    pub deleted_at: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Entry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
#[async_trait]
pub trait TrashStore: Send + Sync {
    async fn list(&self, actor: &str, company: &str, page: i64) -> Result<Page>;
    async fn execute(&self, actor: &str, company: &str, command: Command) -> Result<Entry>;
}

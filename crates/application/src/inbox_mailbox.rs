pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub revision: i64,
    pub enabled: bool,
    #[serde(default)]
    pub automatic: bool,
    pub provider: Option<String>,
    pub uid_validity: Option<u32>,
    pub cursor: u32,
    pub next_run_at: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configure {
    pub revision: i64,
    pub enabled: bool,
    #[serde(default)]
    pub automatic: bool,
    pub provider: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Run {
    pub request_id: String,
    pub status: String,
    pub error: Option<String>,
    pub messages: i32,
    pub attachments: i32,
    pub rejected: i32,
    pub uid_validity: Option<u32>,
    pub cursor: u32,
    pub created_at: String,
}
#[async_trait]
pub trait MailboxStore: Send + Sync {
    async fn settings(&self, a: &str, c: &str) -> Result<Settings>;
    async fn configure(&self, a: &str, c: &str, input: Configure) -> Result<Settings>;
    async fn sync(&self, a: &str, c: &str, request_id: &str) -> Result<Run>;
    async fn history(&self, a: &str, c: &str) -> Result<Vec<Run>>;
}

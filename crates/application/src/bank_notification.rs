pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub account_id: String,
    pub revision: i64,
    pub enabled: bool,
    pub format: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intake {
    pub request_id: String,
    pub message_id: String,
    pub sender: String,
    pub file: crate::banking::StatementInput,
}
#[derive(Serialize)]
pub struct Diagnostic {
    pub message_id: String,
    pub sender: String,
    pub new_count: i32,
    pub duplicate_count: i32,
    pub created_at: String,
}
#[async_trait]
pub trait NotificationStore: Send + Sync {
    async fn settings(&self, a: &str, c: &str, account: &str) -> Result<Settings>;
    async fn configure(&self, a: &str, c: &str, value: Settings) -> Result<Settings>;
    async fn receive(
        &self,
        a: &str,
        c: &str,
        input: Intake,
    ) -> Result<crate::banking::ImportPreview>;
    async fn history(&self, a: &str, c: &str, account: &str) -> Result<Vec<Diagnostic>>;
}

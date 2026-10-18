pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub revision: i64,
    pub filename: String,
    pub media_type: String,
    pub size: i64,
    pub sha256: String,
    pub archived: bool,
    pub received_id: Option<String>,
    #[serde(default)]
    pub received_kind: Option<String>,
    pub created_at: String,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub query: String,
    pub page: Option<i64>,
    pub archived: bool,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Document>,
    pub total: i64,
    pub page: i64,
}
pub use crate::attachment::Upload;
#[derive(Serialize, Deserialize)]
pub struct Command {
    pub request_id: String,
    pub id: String,
    pub revision: i64,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Archive { archived: bool },
    Convert { data: Box<crate::received::Data> },
    Attach { received_id: String },
    Detach,
}
#[derive(Serialize)]
pub struct Audit {
    pub id: String,
    pub revision: i64,
    pub action: String,
    pub created_at: String,
}
#[async_trait]
pub trait InboxStore: Send + Sync {
    async fn settings(&self, a: &str, c: &str) -> Result<Settings>;
    async fn configure(&self, a: &str, c: &str, settings: Settings) -> Result<Settings>;
    async fn mail(&self, a: &str, c: &str, input: MailIntake) -> Result<MailResult>;
    async fn mail_history(&self, a: &str, c: &str) -> Result<Vec<MailDiagnostic>>;
    async fn extract_provider(&self, a: &str, c: &str, id: &str) -> Result<crate::received::Data>;

    async fn history(&self, a: &str, c: &str, id: &str) -> Result<Vec<Audit>>;
    async fn extract(&self, a: &str, c: &str, id: &str) -> Result<crate::received::Data>;
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn upload(&self, a: &str, c: &str, input: Upload) -> Result<Document>;
    async fn download(&self, a: &str, c: &str, id: &str) -> Result<(Document, Vec<u8>)>;
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Document>;
}

/// Explicit company opt-ins; provider names refer to trusted installation config.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub revision: i64,
    pub email_enabled: bool,
    pub extraction_provider: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MailIntake {
    pub request_id: String,
    pub message_id: String,
    pub sender: String,
    pub subject: String,
    pub attachments: Vec<crate::attachment::Upload>,
}
#[derive(Serialize, Deserialize)]
pub struct MailResult {
    pub message_id: String,
    pub documents: Vec<Document>,
}
#[derive(Serialize)]
pub struct MailDiagnostic {
    pub message_id: String,
    pub sender: String,
    pub subject: String,
    pub status: String,
    pub attachment_count: i32,
    pub created_at: String,
}

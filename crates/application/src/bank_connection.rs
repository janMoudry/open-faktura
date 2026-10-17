//! User-configured, read-only bank data connections. Secrets are input-only.
pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub label: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Run {
    pub kind: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub error_code: Option<String>,
    pub imported: i32,
    pub duplicates: i32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Status {
    pub matching_enabled: bool,
    pub matching_next_at: Option<String>,
    pub matching_error: Option<String>,
    pub matching_last: Option<MatchingRun>,
    pub schedule_enabled: bool,
    pub interval_minutes: i32,
    pub next_run_at: Option<String>,
    pub schedule_error: Option<String>,
    pub revision: i64,
    pub provider: Option<String>,
    pub configured: bool,
    pub busy: bool,
    pub last_run: Option<Run>,
    pub last_sync_at: Option<String>,
    pub providers: Vec<Provider>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct MatchingRun {
    pub at: String,
    pub checked: i32,
    pub matched: i32,
}
#[derive(Serialize, Deserialize)]
pub struct Command {
    pub account_id: String,
    pub revision: i64,
    pub request_id: String,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Save {
        provider: String,
        secret: Option<String>,
    },
    Schedule {
        enabled: bool,
        interval_minutes: i32,
    },
    Matching {
        enabled: bool,
    },
    Disconnect,
    Test,
    Sync,
}
#[async_trait]
pub trait ConnectionStore: Send + Sync {
    async fn status(&self, actor: &str, company: &str, account: &str) -> Result<Status>;
    async fn execute(&self, actor: &str, company: &str, command: Command) -> Result<Status>;
}

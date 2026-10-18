//! P20 company lifecycle. Policies are Open Faktura decisions, not iDoklad parity.
pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    AddMember { email: String, role: Role },
    SetRole { user_id: String, role: Role },
    RemoveMember { user_id: String },
    Transfer { user_id: String },
    Leave,
    Restore,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Editor,
    Viewer,
}
impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Editor => "editor",
            Self::Viewer => "viewer",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub role: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub company_id: String,
    pub revision: i64,
    pub deleted: bool,
    pub members: Vec<Member>,
}
#[async_trait]
pub trait AdministrationStore: Send + Sync {
    async fn list(&self, actor: &str, company: &str) -> Result<Snapshot>;
    async fn execute(&self, actor: &str, company: &str, command: Command) -> Result<Snapshot>;
    async fn trash(&self, actor: &str) -> Result<Vec<Snapshot>>;
}

/// Portable company data, not a PostgreSQL restore backup. Credentials are omitted.
pub struct Archive {
    pub bytes: Vec<u8>,
    pub file_name: String,
}
#[async_trait]
pub trait ArchiveStore: Send + Sync {
    async fn export(&self, actor: &str, company: &str) -> Result<Archive>;
}

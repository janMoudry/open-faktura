pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Invoice,
    Sales,
    Received,
}
impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invoice => "invoice",
            Self::Sales => "sales",
            Self::Received => "received",
        }
    }
    pub fn table(&self) -> &'static str {
        match self {
            Self::Invoice => "invoice_draft",
            Self::Sales => "sales_document",
            Self::Received => "received_invoice",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub kind: Kind,
    pub id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub kind: Kind,
    pub id: String,
    pub revision: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub targets: Vec<Version>,
    pub lock: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unlock {
    pub request_id: String,
    pub target: Target,
    pub export_id: String,
    pub reason: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub actor_id: String,
    pub sha256: String,
    pub created_at: String,
    pub targets: Vec<Version>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct History {
    pub exports: Vec<Record>,
    pub locked_by_export: Option<String>,
}
pub struct File {
    pub bytes: Vec<u8>,
    pub filename: String,
}
#[async_trait]
pub trait ExportStore: Send + Sync {
    async fn create(&self, actor: &str, company: &str, request: Request) -> Result<File>;
    async fn file(&self, actor: &str, company: &str, id: &str) -> Result<File>;
    async fn history(&self, actor: &str, company: &str, target: Target) -> Result<History>;
    async fn unlock(&self, actor: &str, company: &str, command: Unlock) -> Result<History>;
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub logo_base64: Option<String>,
    pub signature_base64: Option<String>,
    pub footer_cs: String,
    pub footer_en: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub revision: i64,
    pub data: Style,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    pub data: Style,
}
#[async_trait]
pub trait StyleStore: Send + Sync {
    async fn get(&self, actor: &str, company: &str) -> Result<Snapshot>;
    async fn set(&self, actor: &str, company: &str, command: Command) -> Result<Snapshot>;
}

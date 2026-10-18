pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    BulkPdf,
    Rtf,
    DeliveryNote,
    PaymentConfirmation,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub ids: Vec<String>,
    pub kind: Kind,
    pub language: String,
}
pub struct File {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub media_type: String,
}
#[async_trait]
pub trait OutputStore: Send + Sync {
    async fn render(&self, actor: &str, company: &str, request: Request) -> Result<File>;
}

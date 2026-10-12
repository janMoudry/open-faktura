pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
pub use faktura_domain::import::Record;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prepare {
    pub request_id: String,
    pub source: String,
    pub agenda: String,
    pub records: Vec<Record>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub index: i32,
    pub kind: String,
    pub key: String,
    pub label: String,
    pub status: String,
    pub fields: Vec<String>,
    pub target_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyTotal {
    pub currency: String,
    pub source: String,
    pub imported: String,
    pub skipped: String,
    pub payments: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub id: String,
    pub source: String,
    pub agenda: String,
    pub status: String,
    pub rows: Vec<Row>,
    pub limitations: Vec<String>,
    pub totals: Vec<CurrencyTotal>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apply {
    pub skip: Vec<i32>,
}
#[async_trait]
pub trait ImportStore: Send + Sync {
    async fn prepare(&self, actor: &str, company: &str, input: Prepare) -> Result<Batch>;
    async fn get(&self, actor: &str, company: &str, id: &str) -> Result<Batch>;
    async fn list(&self, actor: &str, company: &str) -> Result<Vec<Batch>>;
    async fn apply(&self, actor: &str, company: &str, id: &str, input: Apply) -> Result<Batch>;
}
pub async fn prepare(
    store: &dyn ImportStore,
    actor: &str,
    company: &str,
    input: Prepare,
) -> Result<Batch> {
    if uuid::Uuid::parse_str(&input.request_id).is_err()
        || !["idoklad", "fakturaonline", "generic"].contains(&input.source.as_str())
        || input.agenda.trim().is_empty()
        || input.agenda.len() > 200
        || input.agenda.chars().any(char::is_control)
        || input.records.is_empty()
        || input.records.len() > 500
    {
        return Err(Error::Invalid(vec!["import".into()]));
    }
    store.prepare(actor, company, input).await
}
pub async fn apply(
    store: &dyn ImportStore,
    actor: &str,
    company: &str,
    id: &str,
    input: Apply,
) -> Result<Batch> {
    if input.skip.len() > 500 {
        return Err(Error::Invalid(vec!["skip".into()]));
    }
    store.apply(actor, company, id, input).await
}

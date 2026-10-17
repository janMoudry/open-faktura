use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use faktura_domain::invoice::{DraftData, Totals};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub request_id: String,
    pub revision: i64,
    pub advance_id: String,
    pub payment_id: String,
    pub issued_on: chrono::NaiveDate,
    pub description: String,
    pub language: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub number: String,
    pub company_id: String,
    pub advance_id: String,
    pub advance_number: String,
    pub payment_id: String,
    pub data: DraftData,
    pub totals: Totals,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Document>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub query: String,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
#[async_trait]
pub trait TaxReceiptStore: Send + Sync {
    async fn preview(&self, a: &str, c: &str, d: &str, p: &str) -> Result<Document>;
    async fn issue(&self, a: &str, c: &str, cmd: Issue) -> Result<Document>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Document>;
    async fn pdf(&self, a: &str, c: &str, id: &str) -> Result<Vec<u8>>;
    async fn list(&self, a: &str, c: &str, s: Search) -> Result<Page>;
}
pub async fn issue(s: &dyn TaxReceiptStore, a: &str, c: &str, mut cmd: Issue) -> Result<Document> {
    cmd.description = cmd.description.trim().into();
    if uuid::Uuid::parse_str(&cmd.request_id).is_err()
        || cmd.revision < 1
        || cmd.description.is_empty()
        || cmd.description.chars().count() > 500
        || cmd.description.chars().any(char::is_control)
        || !["cs", "en"].contains(&cmd.language.as_str())
        || !("1900-01-01"..="9999-12-31").contains(&cmd.issued_on.to_string().as_str())
    {
        return Err(Error::Invalid(vec!["tax_receipt".into()]));
    }
    s.issue(a, c, cmd).await
}
pub async fn list(s: &dyn TaxReceiptStore, a: &str, c: &str, search: Search) -> Result<Page> {
    if search.query.len() > 200
        || !(1..=10000).contains(&search.page.unwrap_or(1))
        || ![10, 25, 50, 100].contains(&search.page_size.unwrap_or(25))
    {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    s.list(a, c, search).await
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub kind: Option<String>,
    pub catalog_id: Option<String>,
    pub query: String,
    pub page: Option<i64>,
    pub source_page: Option<i64>,
}
impl Search {
    pub fn validate(&self) -> Result<()> {
        if self.from.zip(self.to).is_some_and(|(a, b)| a > b)
            || self.query.len() > 200
            || self
                .kind
                .as_deref()
                .is_some_and(|k| !matches!(k, "invoice" | "retail" | "credit_note"))
            || self
                .catalog_id
                .as_ref()
                .is_some_and(|id| id.parse::<i64>().map_or(true, |v| v <= 0))
            || !(1..=10000).contains(&self.page.unwrap_or(1))
            || !(1..=10000).contains(&self.source_page.unwrap_or(1))
        {
            return Err(Error::Invalid(vec!["item_sales".into()]));
        }
        Ok(())
    }
}
#[derive(Serialize)]
pub struct Item {
    pub catalog_id: String,
    pub name: String,
    pub code: String,
    pub unit: String,
    pub currency: String,
    pub quantity: String,
    pub net: String,
    pub gross: String,
}
#[derive(Serialize)]
pub struct Source {
    pub document_id: String,
    pub number: String,
    pub kind: String,
    pub issued_on: NaiveDate,
    pub description: String,
    pub unit: String,
    pub currency: String,
    pub quantity: String,
    pub net: String,
    pub gross: String,
}
#[derive(Serialize)]
pub struct Total {
    pub currency: String,
    pub net: String,
    pub gross: String,
}
#[derive(Serialize)]
pub struct Report {
    pub items: Vec<Item>,
    pub totals: Vec<Total>,
    pub total: i64,
    pub page: i64,
    pub sources: Vec<Source>,
    pub source_total: i64,
    pub source_page: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    pub search: Search,
    pub format: crate::export::Format,
    pub language: crate::export::Language,
}
#[async_trait]
pub trait ItemSalesStore: Send + Sync {
    async fn report(&self, a: &str, c: &str, q: Search) -> Result<Report>;
    async fn export(&self, a: &str, c: &str, q: ExportRequest) -> Result<crate::export::File>;
}

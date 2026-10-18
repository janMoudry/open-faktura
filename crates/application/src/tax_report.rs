use crate::company::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub year: Option<i32>,
    #[serde(default)]
    pub quarterly: bool,
    #[serde(default)]
    pub turnover_sources: bool,
    pub page: Option<i64>,
}
#[derive(Serialize)]
pub struct Period {
    pub period: String,
    pub output: String,
    pub input: String,
    pub difference: String,
}
#[derive(Serialize)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub number: String,
    pub reported_on: chrono::NaiveDate,
    pub amount: String,
    pub side: String,
}
#[derive(Serialize)]
pub struct Report {
    pub year: i32,
    pub tax_mode: String,
    pub turnover: String,
    pub output: String,
    pub input: String,
    pub difference: String,
    pub periods: Vec<Period>,
    pub items: Vec<Entry>,
    pub total: i64,
    pub page: i64,
}
#[async_trait]
pub trait TaxReportStore: Send + Sync {
    async fn report(&self, actor: &str, company: &str, search: Search) -> Result<Report>;
}

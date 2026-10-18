pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Valuation {
    #[default]
    Manual,
    Automatic,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub valuation: Valuation,
    pub as_of: Option<NaiveDate>,
    pub query: String,
    pub page: Option<i64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostCommand {
    pub request_id: String,
    pub catalog_id: String,
    pub revision: i64,
    pub effective_on: NaiveDate,
    pub unit_cost: Option<String>,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
pub struct Cost {
    pub catalog_id: String,
    pub revision: i64,
    pub effective_on: NaiveDate,
    pub unit_cost: Option<String>,
    pub reason: String,
}
#[derive(Serialize)]
pub struct Item {
    pub issued_value: Option<String>,
    pub correction_stock_value: Option<String>,
    pub correction_issued_value: Option<String>,
    pub catalog_id: String,
    pub name: String,
    pub code: String,
    pub unit: String,
    pub archived: bool,
    pub quantity: String,
    pub unit_cost: Option<String>,
    pub value: Option<String>,
    pub cost_revision: i64,
}
#[derive(Serialize)]
pub struct Report {
    pub as_of: NaiveDate,
    pub items: Vec<Item>,
    pub total: i64,
    pub page: i64,
    pub known_value: String,
    pub unvalued_count: i64,
    pub negative_count: i64,
}
#[async_trait]
pub trait StockReportStore: Send + Sync {
    async fn report(&self, a: &str, c: &str, q: Search) -> Result<Report>;
    async fn set_cost(&self, a: &str, c: &str, cmd: CostCommand) -> Result<Cost>;
    async fn costs(&self, a: &str, c: &str, catalog: &str) -> Result<Vec<Cost>>;
}

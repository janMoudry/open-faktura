pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Card {
    pub catalog_id: String,
    pub revision: i64,
    pub opened_on: chrono::NaiveDate,
    pub quantity: String,
    pub item: faktura_domain::catalog::CatalogData,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Movement {
    #[serde(default)]
    pub acquisition_extra_cost: Option<String>,
    #[serde(default)]
    pub acquisition_unit_cost: Option<String>,
    #[serde(default)]
    pub acquisition_correction: bool,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub source_line: Option<String>,
    pub id: String,
    pub catalog_id: String,
    pub quantity: String,
    pub booked_on: chrono::NaiveDate,
    pub kind: String,
    pub reason: String,
    pub item: faktura_domain::catalog::CatalogData,
    pub reverses_id: Option<String>,
}
#[derive(Serialize)]
pub struct Summary {
    pub opening: String,
    pub received: String,
    pub issued: String,
    pub adjustments: String,
    pub closing: String,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Movement>,
    pub summary: Option<Summary>,
    pub total: i64,
    pub page: i64,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub catalog_id: Option<String>,
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
    pub page: Option<i64>,
}
#[derive(Serialize, Deserialize)]
pub struct Command {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_unit_cost: Option<String>,
    pub request_id: String,
    pub catalog_id: String,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Open {
        quantity: String,
        booked_on: chrono::NaiveDate,
        reason: String,
    },
    Move {
        quantity: String,
        booked_on: chrono::NaiveDate,
        reason: String,
    },
    Count {
        quantity: String,
        revision: i64,
        reason: String,
    },
    Reverse {
        movement_id: String,
        reason: String,
    },
}
#[async_trait]
pub trait StockStore: Send + Sync {
    async fn cards(&self, a: &str, c: &str) -> Result<Vec<Card>>;
    async fn movements(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Movement>;
}

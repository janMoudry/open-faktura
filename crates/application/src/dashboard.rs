use crate::company::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub year: Option<i32>,
    #[serde(default)]
    pub all_years: bool,
}
#[derive(Serialize, Deserialize, Default)]
pub struct Balance {
    pub gross: String,
    pub net: String,
    pub settled: String,
    pub outstanding: String,
}
#[derive(Serialize, Deserialize)]
pub struct Age {
    pub side: String,
    pub bucket: String,
    pub amount: String,
    pub count: i64,
}
#[derive(Serialize, Deserialize)]
pub struct Partner {
    pub contact_id: Option<String>,
    pub name: String,
    pub amount: String,
    pub count: i64,
}
#[derive(Serialize)]
pub struct Dashboard {
    pub today: String,
    pub year: i32,
    pub sales: Balance,
    pub purchases: Balance,
    pub aging: Vec<Age>,
    pub customers: Vec<Partner>,
    pub debtors: Vec<Partner>,
}
#[async_trait]
pub trait DashboardStore: Send + Sync {
    async fn dashboard(&self, actor: &str, company: &str, search: Search) -> Result<Dashboard>;
}

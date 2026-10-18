use crate::company::{CompanyError, Result};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Period {
    pub from: NaiveDate,
    pub to: NaiveDate,
}
impl Period {
    pub fn validate(&self) -> Result<()> {
        if self.from > self.to
            || !(1900..=9999).contains(&self.from.year())
            || !(1900..=9999).contains(&self.to.year())
        {
            Err(CompanyError::Invalid(vec!["period".into()]))
        } else {
            Ok(())
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub status: String,
    pub page: Option<i64>,
}
impl Search {
    pub fn validate(&self) -> Result<()> {
        Period {
            from: self.from,
            to: self.to,
        }
        .validate()?;
        if self.query.len() > 200
            || !(1..=10000).contains(&self.page.unwrap_or(1))
            || !["", "all", "cash", "card", "bank_transfer"].contains(&self.method.as_str())
            || !["", "active", "reversed", "all"].contains(&self.status.as_str())
        {
            return Err(CompanyError::Invalid(vec!["query".into()]));
        }
        Ok(())
    }
}
#[derive(Serialize)]
pub struct Entry {
    pub currency: String,
    pub document_id: String,
    pub number: String,
    pub kind: faktura_domain::received::Kind,
    pub supplier: String,
    pub description: String,
    pub payment: faktura_domain::received::Payment,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Entry>,
    pub total: i64,
    pub page: i64,
}
#[derive(Serialize)]
pub struct Overview {
    pub purchases: String,
    pub expenses: String,
    pub outstanding: String,
    pub overdue: String,
}
#[async_trait]
pub trait PurchaseReportStore: Send + Sync {
    async fn payments(&self, actor: &str, company: &str, query: Search) -> Result<Page>;
    async fn overview(&self, actor: &str, company: &str, period: Period) -> Result<Overview>;
}

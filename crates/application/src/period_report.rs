use crate::company::{CompanyError, Result};
use async_trait::async_trait;
use chrono::{Datelike, Months, NaiveDate};
use serde::{Deserialize, Serialize};
#[derive(Default, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueMode {
    #[default]
    Gross,
    Net,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub page: Option<i64>,
    #[serde(default)]
    pub value_mode: ValueMode,
    #[serde(default)]
    pub cumulative: bool,
    #[serde(default)]
    pub compare_previous: bool,
}
impl Search {
    pub fn previous_period(&self) -> (NaiveDate, NaiveDate) {
        // Chrono clamps leap day to the last valid day of the target month.
        (
            self.from
                .checked_sub_months(Months::new(12))
                .expect("validated year"),
            self.to
                .checked_sub_months(Months::new(12))
                .expect("validated year"),
        )
    }
    pub fn validate(&self) -> Result<()> {
        crate::purchase_report::Period {
            from: self.from,
            to: self.to,
        }
        .validate()?;
        if (self.to - self.from).num_days() > 366
            || (self.compare_previous
                && (self.from.year() == 1900 || {
                    let (from, to) = self.previous_period();
                    (to - from).num_days() > 366
                }))
            || !(1..=10000).contains(&self.page.unwrap_or(1))
        {
            return Err(CompanyError::Invalid(vec!["period".into()]));
        }
        Ok(())
    }
}
#[derive(Serialize)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub number: String,
    pub reported_on: NaiveDate,
    pub amount: String,
    pub side: String,
}
#[derive(Serialize)]
pub struct Month {
    pub month: String,
    pub sales: String,
    pub purchases: String,
}
#[derive(Serialize)]
pub struct Comparison {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub sales: String,
    pub purchases: String,
    pub sales_difference: String,
    pub purchases_difference: String,
    pub months: Vec<Month>,
}
#[derive(Serialize)]
pub struct Report {
    pub sales: String,
    pub purchases: String,
    pub currency: String,
    pub comparison: Option<Comparison>,
    pub months: Vec<Month>,
    pub items: Vec<Entry>,
    pub total: i64,
    pub page: i64,
}
#[async_trait]
pub trait PeriodReportStore: Send + Sync {
    async fn report(&self, actor: &str, company: &str, q: Search) -> Result<Report>;
}

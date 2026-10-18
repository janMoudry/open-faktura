//! Collection communication is separate from invoice issue and payment registration.
pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub revision: i64,
    pub reminders: bool,
    pub thanks: bool,
    pub first_days: i32,
    pub repeat_days: i32,
    pub minimum_total: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            revision: 0,
            reminders: false,
            thanks: false,
            first_days: 7,
            repeat_days: 7,
            minimum_total: "0.00".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        let amount = rust_decimal::Decimal::from_str(&self.minimum_total);
        if self.revision < 0
            || !(1..=365).contains(&self.first_days)
            || !(1..=365).contains(&self.repeat_days)
            || amount.is_err()
            || amount.is_ok_and(|v| {
                v.is_sign_negative()
                    || v.scale() > 2
                    || v > rust_decimal::Decimal::from(1_000_000_000_000i64)
            })
        {
            return Err(Error::Invalid(vec!["communication_settings".into()]));
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Preview {
    pub automation_error: Option<String>,
    pub purpose: String,
    pub eligible: bool,
    pub paused: bool,
    pub settlement_revision: i64,
    pub revision: i64,
    pub recipient: String,
    pub subject: String,
    pub body: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pause {
    pub paused: bool,
}
#[async_trait]
pub trait CommunicationStore: Send + Sync {
    async fn settings(&self, a: &str, c: &str) -> Result<Settings>;
    async fn save(&self, a: &str, c: &str, value: Settings) -> Result<Settings>;
    async fn preview(&self, a: &str, c: &str, id: &str, purpose: &str) -> Result<Preview>;
    async fn pause(&self, a: &str, c: &str, id: &str, value: Pause) -> Result<Pause>;
}

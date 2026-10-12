use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentInput {
    pub amount: String,
    pub paid_on: chrono::NaiveDate,
    pub method: String,
    pub note: String,
}
impl PaymentInput {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let parts: Vec<_> = self.amount.split('.').collect();
        let amount = Decimal::from_str(&self.amount).ok();
        if self.amount.len() > 15
            || parts.len() > 2
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
            || parts.get(1).is_some_and(|p| p.len() > 2)
            || amount.is_none_or(|v| {
                v <= Decimal::ZERO || v > Decimal::from_str("999999999999.99").unwrap()
            })
        {
            errors.push("amount".into());
        }
        if !["cash", "bank_transfer"].contains(&self.method.as_str()) {
            errors.push("method".into());
        }
        if self.note.chars().count() > 500 || self.note.chars().any(|c| c.is_control() && c != '\n')
        {
            errors.push("note".into());
        }
        errors
    }
}

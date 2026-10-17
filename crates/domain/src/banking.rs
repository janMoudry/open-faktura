use crate::received::amount;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountData {
    pub name: String,
    pub iban: String,
    pub currency: String,
}
impl AccountData {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().into();
        self.iban = self
            .iban
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_uppercase)
            .collect();
    }
    pub fn validate(&self) -> Vec<String> {
        let mut e = vec![];
        if self.name.is_empty() || !text(&self.name, 100) {
            e.push("name".into());
        }
        if !crate::company::valid_iban(&self.iban) {
            e.push("iban".into());
        }
        if self.currency != "CZK" {
            e.push("currency".into());
        }
        e
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MovementData {
    pub external_id: String,
    pub booked_on: NaiveDate,
    pub amount: String,
    pub currency: String,
    pub variable_symbol: String,
    pub counterparty: String,
    pub description: String,
}
fn text(v: &str, max: usize) -> bool {
    v.chars().count() <= max && !v.chars().any(char::is_control)
}
impl MovementData {
    pub fn normalize(&mut self) {
        self.external_id = self.external_id.trim().into();
        self.variable_symbol = self.variable_symbol.trim().into();
        self.counterparty = self.counterparty.trim().into();
        self.description = self.description.trim().into();
        if let Some(a) = amount(&self.amount) {
            self.amount = crate::received::money(a);
        }
    }
    pub fn validate(&self) -> Vec<String> {
        let mut e = vec![];
        if self.external_id.is_empty() || !text(&self.external_id, 200) {
            e.push("external_id".into());
        }
        if amount(&self.amount).is_none_or(|v| v.is_zero()) {
            e.push("amount".into());
        }
        if !(1900..=9999).contains(&self.booked_on.year()) {
            e.push("booked_on".into());
        }
        if self.currency != "CZK" {
            e.push("currency".into());
        }
        if self.variable_symbol.len() > 10
            || !self.variable_symbol.bytes().all(|b| b.is_ascii_digit())
        {
            e.push("variable_symbol".into());
        }
        if !text(&self.counterparty, 200) || !text(&self.description, 500) {
            e.push("text".into());
        }
        e
    }
}

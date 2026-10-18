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
        if self.iban.parse::<iban::Iban>().is_err() {
            e.push("iban".into());
        }
        if !crate::exchange::CURRENCIES.contains(&self.currency.as_str()) {
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
        if !crate::exchange::CURRENCIES.contains(&self.currency.as_str()) {
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

#[cfg(test)]
mod iban_tests {
    use super::*;

    #[test]
    fn international_accounts_validate_country_format_and_checksum() {
        // Public example identifiers, not accounts contacted by these tests.
        for iban in [
            "CZ6508000000192000145399",
            "SK3112000000198742637541",
            "DE89370400440532013000",
            "GB82WEST12345698765432",
            "FR1420041010050500013M02606",
            "NL91ABNA0417164300",
            "CH9300762011623852957",
        ] {
            let data = AccountData {
                name: "Test".into(),
                iban: iban.into(),
                currency: "EUR".into(),
            };
            assert!(data.validate().is_empty(), "{iban}");
        }
        for iban in [
            "",
            "DE89370400440532013001",
            "DE8937040044053201300",
            "ZZ89370400440532013000",
            "GB82WES112345698765432",
            "DE89💰",
            "DE89370400440532013000;",
        ] {
            let data = AccountData {
                name: "Test".into(),
                iban: iban.into(),
                currency: "EUR".into(),
            };
            assert!(data.validate().contains(&"iban".into()), "{iban}");
        }
        let mut data = AccountData {
            name: " Test ".into(),
            iban: "de89 3704 0044 0532 0130 00".into(),
            currency: "USD".into(),
        };
        data.normalize();
        assert_eq!(data.iban, "DE89370400440532013000");
        assert!(data.validate().is_empty());
        assert!(crate::company::valid_iban(&data.iban));
    }
}

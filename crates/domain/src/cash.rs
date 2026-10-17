use crate::received::amount;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccountData {
    pub name: String,
    pub currency: String,
    pub opening_on: NaiveDate,
    pub opening_balance: String,
}
impl AccountData {
    pub fn normalize(&mut self) {
        self.name = self.name.trim().into();
    }
    pub fn validate(&self) -> Vec<String> {
        let mut e = vec![];
        if self.name.is_empty()
            || self.name.chars().count() > 100
            || self.name.chars().any(char::is_control)
        {
            e.push("name".into());
        }
        if self.currency != "CZK" {
            e.push("currency".into());
        }
        if amount(&self.opening_balance).is_none() {
            e.push("opening_balance".into());
        }
        if !(1900..=9999).contains(&self.opening_on.year()) {
            e.push("opening_on".into());
        }
        e
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MovementData {
    pub booked_on: NaiveDate,
    pub amount: String,
    pub currency: String,
    pub variable_symbol: String,
    pub counterparty: String,
    pub description: String,
    pub person: String,
}
impl MovementData {
    pub fn normalize(&mut self) {
        self.counterparty = self.counterparty.trim().into();
        self.person = self.person.trim().into();
        self.description = self.description.trim().into();
        self.variable_symbol = self.variable_symbol.trim().into();
        if let Some(a) = amount(&self.amount) {
            self.amount = crate::received::money(a);
        }
    }
    pub fn validate(&self) -> Vec<String> {
        let d = crate::banking::MovementData {
            external_id: "cash".into(),
            booked_on: self.booked_on,
            amount: self.amount.clone(),
            currency: self.currency.clone(),
            variable_symbol: self.variable_symbol.clone(),
            counterparty: self.counterparty.clone(),
            description: self.description.clone(),
        };
        let mut e = d.validate();
        if self.person.chars().count() > 200 || self.person.chars().any(char::is_control) {
            e.push("person".into());
        }
        e
    }
}

/// One standalone cash item. The signed input amount is a price, not a payment.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxInput {
    pub price_mode: crate::invoice::PriceMode,
    pub vat_percent: String,
    pub tax_on: NaiveDate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxRequest {
    pub amount: String,
    pub tax: TaxInput,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxSnapshot {
    pub input_amount: String,
    pub tax: TaxInput,
    pub calculation_version: String,
    pub amounts: crate::invoice::Amounts,
}
pub fn calculate_tax(request: &TaxRequest) -> Result<TaxSnapshot, Vec<String>> {
    use crate::invoice::*;
    let value = amount(&request.amount)
        .filter(|v| !v.is_zero())
        .ok_or_else(|| vec!["cash_tax_amount".into()])?;
    if !(1900..=9999).contains(&request.tax.tax_on.year()) {
        return Err(vec!["cash_tax_date".into()]);
    }
    let calculation = Calculation {
        calculation_version: VERSION.into(),
        currency: "CZK".into(),
        tax_mode: TaxMode::VatPayer,
        price_mode: request.tax.price_mode,
        discount_percent: "0".into(),
        rounding: Rounding::None,
        lines: vec![Line {
            catalog_id: None,
            id: "cash".into(),
            description: "cash".into(),
            unit: "".into(),
            quantity: "1".into(),
            unit_price: crate::received::money(value.abs()),
            discount_percent: "0".into(),
            vat_percent: request.tax.vat_percent.clone(),
        }],
    };
    let result = calculate(&calculation).map_err(|_| vec!["cash_tax_amount".into()])?;
    if amount(&result.payable).is_none_or(|v| v.is_zero()) {
        return Err(vec!["cash_tax_amount".into()]);
    }
    Ok(TaxSnapshot {
        input_amount: crate::received::money(value),
        tax: request.tax.clone(),
        calculation_version: result.calculation_version,
        amounts: result.amounts,
    })
}

#[cfg(test)]
mod tax_tests {
    use super::*;
    use crate::invoice::PriceMode;
    #[test]
    fn standalone_prices_are_exact_and_signed_only_in_ledger() {
        for (value, mode, rate, net, vat, gross) in [
            ("100.00", PriceMode::Net, "21", "100.00", "21.00", "121.00"),
            (
                "-121.00",
                PriceMode::Gross,
                "21",
                "100.00",
                "21.00",
                "121.00",
            ),
            (
                "112.00",
                PriceMode::Gross,
                "12",
                "100.00",
                "12.00",
                "112.00",
            ),
            ("0.03", PriceMode::Gross, "21", "0.02", "0.01", "0.03"),
            ("5.02", PriceMode::Net, "0", "5.02", "0.00", "5.02"),
        ] {
            let result = calculate_tax(&TaxRequest {
                amount: value.into(),
                tax: TaxInput {
                    price_mode: mode,
                    vat_percent: rate.into(),
                    tax_on: "2026-10-01".parse().unwrap(),
                },
            })
            .unwrap();
            assert_eq!(
                (
                    result.amounts.net.as_str(),
                    result.amounts.vat.as_str(),
                    result.amounts.gross.as_str()
                ),
                (net, vat, gross)
            );
        }
        for value in ["0", "1.001", "1e3", "NaN", "1000000000.01"] {
            assert!(calculate_tax(&TaxRequest {
                amount: value.into(),
                tax: TaxInput {
                    price_mode: PriceMode::Net,
                    vat_percent: "21".into(),
                    tax_on: "2026-10-01".parse().unwrap()
                }
            })
            .is_err());
        }
    }
}

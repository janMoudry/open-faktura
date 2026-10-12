//! P12-01: reusable CZK price-list entries, independent of invoice snapshots.
use crate::invoice::{decimal, Line, PriceMode, TaxMode};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogData {
    pub name: String,
    pub code: String,
    pub barcode: String,
    pub unit: String,
    pub quantity: String,
    pub unit_price: String,
    pub currency: String,
    pub price_mode: PriceMode,
    pub vat_percent: String,
}
impl Default for CatalogData {
    fn default() -> Self {
        Self {
            name: String::new(),
            code: String::new(),
            barcode: String::new(),
            unit: String::new(),
            quantity: "1".into(),
            unit_price: "0".into(),
            currency: "CZK".into(),
            price_mode: PriceMode::Net,
            vat_percent: "0".into(),
        }
    }
}
impl CatalogData {
    pub fn normalize_validate(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        for (key, value, max) in [
            ("name", &mut self.name, 200),
            ("code", &mut self.code, 80),
            ("barcode", &mut self.barcode, 80),
            ("unit", &mut self.unit, 20),
        ] {
            *value = value.trim().to_owned();
            if value.chars().count() > max
                || value.chars().any(char::is_control)
                || (key == "name" && value.is_empty())
            {
                errors.push(key.into());
            }
        }
        for (key, value, max, positive) in [
            ("quantity", &mut self.quantity, 1_000_000, true),
            ("unit_price", &mut self.unit_price, 1_000_000_000, false),
        ] {
            *value = value.trim().to_owned();
            match decimal(value, 4, max) {
                Some(n) if !positive || n > Decimal::ZERO => *value = n.normalize().to_string(),
                _ => errors.push(key.into()),
            }
        }
        if self.currency != "CZK" {
            errors.push("currency".into());
        }
        if !["0", "12", "21"].contains(&self.vat_percent.as_str()) {
            errors.push("vat_percent".into());
        }
        errors
    }
    pub fn invoice_line(
        &self,
        id: String,
        price_mode: PriceMode,
        tax_mode: TaxMode,
    ) -> Result<Line, Vec<String>> {
        let mut checked = self.clone();
        let errors = checked.normalize_validate();
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut price =
            Decimal::from_str(&checked.unit_price).map_err(|_| vec!["unit_price".into()])?;
        if tax_mode == TaxMode::VatPayer && self.price_mode != price_mode {
            let rate =
                Decimal::from_str(&self.vat_percent).map_err(|_| vec!["vat_percent".into()])?;
            let factor = Decimal::ONE + rate / Decimal::from(100);
            price = if price_mode == PriceMode::Gross {
                price * factor
            } else {
                price / factor
            };
            price = price.round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero);
        }
        if price > Decimal::from(1_000_000_000) {
            return Err(vec!["unit_price".into()]);
        }
        Ok(Line {
            id,
            description: checked.name,
            unit: checked.unit,
            quantity: checked.quantity,
            unit_price: price.normalize().to_string(),
            discount_percent: "0".into(),
            vat_percent: if tax_mode == TaxMode::NonPayer {
                "0".into()
            } else {
                checked.vat_percent
            },
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_conversion_precision_and_limits() {
        let mut item = CatalogData {
            name: "Precision fixture".into(),
            unit_price: "1".into(),
            price_mode: PriceMode::Gross,
            vat_percent: "21".into(),
            ..Default::default()
        };
        assert_eq!(
            item.invoice_line("a".into(), PriceMode::Net, TaxMode::VatPayer)
                .unwrap()
                .unit_price,
            "0.8264"
        );
        item.price_mode = PriceMode::Net;
        item.unit_price = "0.0005".into();
        item.vat_percent = "12".into();
        assert_eq!(
            item.invoice_line("a".into(), PriceMode::Gross, TaxMode::VatPayer)
                .unwrap()
                .unit_price,
            "0.0006"
        );
        item.unit_price = "1000000000".into();
        assert!(item
            .invoice_line("a".into(), PriceMode::Gross, TaxMode::VatPayer)
            .is_err());
        for price in ["-1", "NaN", "1.00001", "1000000001"] {
            item.unit_price = price.into();
            assert!(item.normalize_validate().contains(&"unit_price".into()));
        }
    }
    #[test]
    fn catalog_validates_and_converts_exact_prices() {
        let mut item = CatalogData {
            name: " Service ".into(),
            unit_price: "121.0000".into(),
            price_mode: PriceMode::Gross,
            vat_percent: "21".into(),
            ..Default::default()
        };
        assert!(item.normalize_validate().is_empty());
        assert_eq!(item.name, "Service");
        assert_eq!(
            item.invoice_line("line".into(), PriceMode::Net, TaxMode::VatPayer)
                .unwrap()
                .unit_price,
            "100"
        );
        assert_eq!(
            item.invoice_line("line".into(), PriceMode::Net, TaxMode::NonPayer)
                .unwrap()
                .unit_price,
            "121"
        );
        item.quantity = "0".into();
        assert!(item.normalize_validate().contains(&"quantity".into()));
        item.quantity = "1e2".into();
        assert!(item.normalize_validate().contains(&"quantity".into()));
    }
}

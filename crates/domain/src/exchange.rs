//! Document-owned CZK conversion; never fetch today's rate for historical documents.
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeRate {
    pub rate: String,
    pub units: String,
}
pub const CURRENCIES: &[&str] = &[
    "CZK", "EUR", "USD", "GBP", "CHF", "PLN", "HUF", "JPY", "SEK", "NOK", "DKK", "CAD", "AUD",
];
pub fn validate(currency: &str, exchange: Option<&ExchangeRate>) -> bool {
    if !CURRENCIES.contains(&currency) {
        return false;
    }
    if currency == "CZK" {
        return exchange.is_none();
    }
    exchange.is_some_and(|r| values(r).is_some())
}
fn values(r: &ExchangeRate) -> Option<(Decimal, Decimal)> {
    let rate = crate::invoice::decimal(&r.rate, 6, 1_000_000)?;
    let units = crate::invoice::decimal(&r.units, 0, 1_000_000)?;
    (rate > Decimal::ZERO && units > Decimal::ZERO).then_some((rate, units))
}
pub fn to_czk(amount: Decimal, currency: &str, exchange: Option<&ExchangeRate>) -> Option<Decimal> {
    if !validate(currency, exchange) {
        return None;
    }
    let value = if let Some(r) = exchange {
        let (rate, units) = values(r)?;
        amount.checked_mul(rate)?.checked_div(units)?
    } else {
        amount
    };
    Some(value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_rates_units_rounding_and_validation() {
        let r = ExchangeRate {
            rate: "15.125".into(),
            units: "100".into(),
        };
        assert_eq!(
            to_czk(Decimal::from(1000), "JPY", Some(&r))
                .unwrap()
                .to_string(),
            "151.25"
        );
        let r = ExchangeRate {
            rate: "25.5".into(),
            units: "1".into(),
        };
        assert_eq!(
            to_czk(Decimal::new(-1, 2), "EUR", Some(&r)).unwrap(),
            Decimal::new(-26, 2)
        );
        assert!(!validate("EUR", None));
        assert!(!validate("CZK", Some(&r)));
        for (rate, units) in [
            ("0", "1"),
            ("25", "0"),
            ("25", "0.5"),
            ("NaN", "1"),
            ("1.1234567", "1"),
        ] {
            assert!(!validate(
                "EUR",
                Some(&ExchangeRate {
                    rate: rate.into(),
                    units: units.into()
                })
            ));
        }
    }
}

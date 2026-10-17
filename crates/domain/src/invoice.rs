//! Explicit Open Faktura preview rules; not a claim of iDoklad calculation parity.
use crate::contact::Address;
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};
pub const VERSION: &str = "of-czk-v1";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PriceMode {
    Net,
    Gross,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaxMode {
    NonPayer,
    VatPayer,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Rounding {
    None,
    WholeCzk,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    pub id: String,
    pub description: String,
    pub unit: String,
    pub quantity: String,
    pub unit_price: String,
    pub discount_percent: String,
    pub vat_percent: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calculation {
    pub calculation_version: String,
    pub currency: String,
    pub tax_mode: TaxMode,
    pub price_mode: PriceMode,
    pub discount_percent: String,
    pub rounding: Rounding,
    pub lines: Vec<Line>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Amounts {
    pub net: String,
    pub vat: String,
    pub gross: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineResult {
    pub id: String,
    pub amounts: Amounts,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaxSummary {
    pub tax_mode: TaxMode,
    pub vat_percent: String,
    pub amounts: Amounts,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Totals {
    pub calculation_version: String,
    pub currency: String,
    pub lines: Vec<LineResult>,
    pub tax_summary: Vec<TaxSummary>,
    pub amounts: Amounts,
    pub rounding: String,
    pub payable: String,
}
/// Owned copies: no live contact/company data is dereferenced by a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartySnapshot {
    pub name: String,
    pub registration_number: String,
    pub vat_number: String,
    pub address: Address,
    pub email: String,
    pub phone: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentSnapshot {
    pub method: String,
    pub bank_account: String,
    pub iban: String,
    pub variable_symbol: String,
}
/// Draft persistence, date and issue validation are implemented in P06/P07.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftData {
    pub contact_id: Option<String>,
    pub supplier: PartySnapshot,
    pub customer: Option<PartySnapshot>,
    pub payment: PaymentSnapshot,
    pub calculation: Calculation,
    pub issued_on: Option<chrono::NaiveDate>,
    pub due_on: Option<chrono::NaiveDate>,
    pub tax_on: Option<chrono::NaiveDate>,
    pub language: String,
    pub order_number: String,
    pub note: String,
}
pub(crate) fn decimal(value: &str, scale: usize, max: i64) -> Option<Decimal> {
    if value.is_empty() || value.len() > 24 {
        return None;
    }
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() > 2
        || parts
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
        || parts.get(1).is_some_and(|s| s.len() > scale)
    {
        return None;
    }
    Decimal::from_str(value)
        .ok()
        .filter(|v| *v <= Decimal::from(max))
}
fn cents(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}
fn money(v: Decimal) -> String {
    let mut v = cents(v);
    v.rescale(2);
    v.to_string()
}
fn amounts(net: Decimal, vat: Decimal) -> Amounts {
    Amounts {
        net: money(net),
        vat: money(vat),
        gross: money(net + vat),
    }
}
pub fn calculate(input: &Calculation) -> Result<Totals, Vec<String>> {
    let mut errors = Vec::new();
    if input.calculation_version != VERSION {
        errors.push("calculation_version".into());
    }
    if input.currency != "CZK" {
        errors.push("currency".into());
    }
    if input.lines.len() > 500 {
        errors.push("lines".into());
    }
    let discount = decimal(&input.discount_percent, 2, 100);
    if discount.is_none() {
        errors.push("discount_percent".into());
    }
    let mut parsed = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for (i, line) in input.lines.iter().enumerate().take(501) {
        let prefix = format!("lines.{i}");
        if line.id.is_empty() || line.id.len() > 64 || !ids.insert(&line.id) {
            errors.push(format!("{prefix}.id"));
        }
        if line.description.chars().count() > 2000 {
            errors.push(format!("{prefix}.description"));
        }
        if line.unit.chars().count() > 30 {
            errors.push(format!("{prefix}.unit"));
        }
        let q = decimal(&line.quantity, 4, 1_000_000);
        let p = decimal(&line.unit_price, 4, 1_000_000_000);
        let d = decimal(&line.discount_percent, 2, 100);
        let rate = match line.vat_percent.as_str() {
            "0" => Some(0),
            "12" => Some(12),
            "21" => Some(21),
            _ => None,
        };
        for (key, valid) in [
            ("quantity", q.is_some()),
            ("unit_price", p.is_some()),
            ("discount_percent", d.is_some()),
            (
                "vat_percent",
                rate.is_some() && (input.tax_mode == TaxMode::VatPayer || rate == Some(0)),
            ),
        ] {
            if !valid {
                errors.push(format!("{prefix}.{key}"));
            }
        }
        if let (Some(q), Some(p), Some(d), Some(rate)) = (q, p, d, rate) {
            parsed.push((q, p, d, rate));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    // Bounded operands above keep all intermediate values within Decimal's 96-bit range.
    let factor = (Decimal::from(100) - discount.unwrap()) / Decimal::from(100);
    let mut buckets: BTreeMap<i32, (Decimal, Decimal)> = BTreeMap::new();
    let mut lines = Vec::new();
    let (mut net_total, mut vat_total) = (Decimal::ZERO, Decimal::ZERO);
    for (line, (q, p, d, rate)) in input.lines.iter().zip(parsed) {
        let source = cents(q * p * ((Decimal::from(100) - d) / Decimal::from(100)) * factor);
        let r = Decimal::from(rate) / Decimal::from(100);
        let (net, vat) = if input.price_mode == PriceMode::Gross {
            let net = cents(source / (Decimal::ONE + r));
            (net, source - net)
        } else {
            (source, cents(source * r))
        };
        net_total += net;
        vat_total += vat;
        let bucket = buckets.entry(rate).or_default();
        bucket.0 += net;
        bucket.1 += vat;
        lines.push(LineResult {
            id: line.id.clone(),
            amounts: amounts(net, vat),
        });
    }
    let gross = net_total + vat_total;
    let payable = if input.rounding == Rounding::WholeCzk {
        gross.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
    } else {
        gross
    };
    Ok(Totals {
        calculation_version: VERSION.into(),
        currency: "CZK".into(),
        lines,
        tax_summary: buckets
            .into_iter()
            .map(|(rate, (net, vat))| TaxSummary {
                tax_mode: input.tax_mode,
                vat_percent: rate.to_string(),
                amounts: amounts(net, vat),
            })
            .collect(),
        amounts: amounts(net_total, vat_total),
        rounding: money(payable - gross),
        payable: money(payable),
    })
}
/// Drafts may omit parties/dates and numeric inputs, but cannot persist malformed values.
pub fn validate_draft(data: &DraftData) -> Vec<String> {
    let mut calc = data.calculation.clone();
    if calc.discount_percent.is_empty() {
        calc.discount_percent = "0".into();
    }
    for line in &mut calc.lines {
        for value in [
            &mut line.quantity,
            &mut line.unit_price,
            &mut line.discount_percent,
        ] {
            if value.is_empty() {
                *value = "0".into();
            }
        }
    }
    let mut errors = calculate(&calc).err().unwrap_or_default();
    for (prefix, party) in [
        ("supplier", Some(&data.supplier)),
        ("customer", data.customer.as_ref()),
    ] {
        if let Some(p) = party {
            for (key, value, max) in [
                ("name", &p.name, 200),
                ("registration_number", &p.registration_number, 32),
                ("vat_number", &p.vat_number, 32),
                ("email", &p.email, 254),
                ("phone", &p.phone, 50),
                ("street", &p.address.street, 200),
                ("city", &p.address.city, 120),
                ("postal_code", &p.address.postal_code, 30),
                ("country", &p.address.country, 2),
            ] {
                if value.chars().count() > max || value.chars().any(char::is_control) {
                    errors.push(format!("{prefix}.{key}"));
                }
            }
        }
    }
    if !["cs", "en"].contains(&data.language.as_str()) {
        errors.push("language".into());
    }
    if !["cash", "bank_transfer"].contains(&data.payment.method.as_str()) {
        errors.push("payment.method".into());
    }
    for (key, value, max) in [
        ("order_number", &data.order_number, 100),
        ("note", &data.note, 4000),
        ("payment.iban", &data.payment.iban, 34),
        ("payment.bank_account", &data.payment.bank_account, 80),
        ("payment.variable_symbol", &data.payment.variable_symbol, 30),
    ] {
        if value.chars().count() > max {
            errors.push(key.into());
        }
    }
    if data
        .contact_id
        .as_ref()
        .is_some_and(|id| id.parse::<i64>().map_or(true, |v| v <= 0))
    {
        errors.push("contact_id".into());
    }
    errors
}

/// P07 supported domestic CZK invoices. These are Open Faktura rules, not inferred SDK behavior.
pub fn validate_issue(data: &DraftData) -> Result<Totals, Vec<String>> {
    let mut errors = validate_draft(data);
    if data.calculation.lines.is_empty() {
        errors.push("lines".into());
    }
    let totals = calculate(&data.calculation)
        .map_err(|e| errors.extend(e))
        .ok();
    for (prefix, party) in [
        ("supplier", Some(&data.supplier)),
        ("customer", data.customer.as_ref()),
    ] {
        if let Some(p) = party {
            for (key, value) in [
                ("name", &p.name),
                ("street", &p.address.street),
                ("city", &p.address.city),
                ("postal_code", &p.address.postal_code),
            ] {
                if value.trim().is_empty() {
                    errors.push(format!("{prefix}.{key}"));
                }
            }
            if p.address.country != "CZ" {
                errors.push(format!("{prefix}.country"));
            }
        } else {
            errors.push(prefix.into());
        }
    }
    if data.supplier.registration_number.trim().is_empty() {
        errors.push("supplier.registration_number".into());
    }
    if data.issued_on.is_none() {
        errors.push("issued_on".into());
    }
    if data.due_on.is_none() || data.due_on < data.issued_on {
        errors.push("due_on".into());
    }
    if data.calculation.tax_mode == TaxMode::VatPayer {
        if data.tax_on.is_none() {
            errors.push("tax_on".into());
        }
        if data.supplier.vat_number.trim().is_empty() {
            errors.push("supplier.vat_number".into());
        }
    }
    for (i, line) in data.calculation.lines.iter().enumerate() {
        if line.description.trim().is_empty() {
            errors.push(format!("lines.{i}.description"));
        }
        if decimal(&line.quantity, 6, 1_000_000_000).is_none_or(|v| v <= Decimal::ZERO) {
            errors.push(format!("lines.{i}.quantity"));
        }
    }
    if data.payment.method == "bank_transfer"
        && data.payment.bank_account.trim().is_empty()
        && data.payment.iban.trim().is_empty()
    {
        errors.push("payment.bank_account".into());
    }
    if !data.payment.iban.is_empty() && !crate::company::valid_iban(&data.payment.iban) {
        errors.push("payment.iban".into());
    }
    if !data
        .payment
        .variable_symbol
        .bytes()
        .all(|c| c.is_ascii_digit())
        || data.payment.variable_symbol.len() > 10
    {
        errors.push("payment.variable_symbol".into());
    }
    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(totals.expect("calculation succeeded"))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(q: &str, price: &str, rate: &str) -> Calculation {
        Calculation {
            calculation_version: VERSION.into(),
            currency: "CZK".into(),
            tax_mode: TaxMode::VatPayer,
            price_mode: PriceMode::Net,
            discount_percent: "0".into(),
            rounding: Rounding::None,
            lines: vec![Line {
                catalog_id: None,
                id: "a".into(),
                description: "".into(),
                unit: "ks".into(),
                quantity: q.into(),
                unit_price: price.into(),
                discount_percent: "0".into(),
                vat_percent: rate.into(),
            }],
        }
    }
    #[test]
    fn independent_examples() {
        // Hand-calculated expectations for our v1 rules, not reference observations.
        let mut i = input("3", "19.995", "21");
        let t = calculate(&i).unwrap();
        assert_eq!(
            (
                t.amounts.net.as_str(),
                t.amounts.vat.as_str(),
                t.payable.as_str()
            ),
            ("59.99", "12.60", "72.59")
        );
        i.price_mode = PriceMode::Gross;
        i.lines[0].unit_price = "121".into();
        i.lines[0].quantity = "1".into();
        i.lines[0].discount_percent = "10".into();
        i.discount_percent = "5".into();
        let t = calculate(&i).unwrap();
        assert_eq!(
            (
                t.amounts.net.as_str(),
                t.amounts.vat.as_str(),
                t.payable.as_str()
            ),
            ("85.50", "17.96", "103.46")
        );
        i.rounding = Rounding::WholeCzk;
        let t = calculate(&i).unwrap();
        assert_eq!(t.rounding, "-0.46");
        assert_eq!(t.payable, "103.00");
        let t = calculate(&input("0.125", "19.995", "12")).unwrap();
        assert_eq!(t.payable, "2.80");
    }
    #[test]
    fn discounts_and_sum_invariants() {
        let mut i = input("1", "0.01", "21");
        i.lines[0].discount_percent = "100".into();
        assert_eq!(calculate(&i).unwrap().payable, "0.00");
        i.lines[0].discount_percent = "100.01".into();
        assert!(calculate(&i).is_err());
        i = input("1", "0.01", "21");
        i.lines = (0..500)
            .map(|n| Line {
                catalog_id: None,
                id: n.to_string(),
                ..i.lines[0].clone()
            })
            .collect();
        let t = calculate(&i).unwrap();
        // Per-line tax is 0.00: distinguish line aggregation from invoice tax recalculation.
        assert_eq!(t.amounts.net, "5.00");
        assert_eq!(t.amounts.vat, "0.00");
        i.lines.reverse();
        assert_eq!(calculate(&i).unwrap().amounts, t.amounts);
        i.lines.push(Line {
            catalog_id: None,
            id: "overflow".into(),
            ..i.lines[0].clone()
        });
        assert!(calculate(&i).is_err());
    }
    #[test]
    fn bounds_and_recap() {
        let mut i = input("1000000", "1000000000", "21");
        i.lines[0].id = "max".into();
        assert!(calculate(&i).is_ok());
        i.lines.push(Line {
            catalog_id: None,
            id: "zero".into(),
            vat_percent: "0".into(),
            ..i.lines[0].clone()
        });
        let t = calculate(&i).unwrap();
        assert_eq!(t.tax_summary.len(), 2);
        assert_eq!(t.payable, "2210000000000000.00");
        i.tax_mode = TaxMode::NonPayer;
        assert!(calculate(&i).is_err());
        for bad in ["NaN", "-1", "1e2", "1,2", "1.12345", "1000000001"] {
            i = input("1", bad, "0");
            assert!(calculate(&i).is_err(), "{bad}");
        }
        i = input("1", "1", "0");
        i.lines.push(i.lines[0].clone());
        assert!(calculate(&i).is_err());
        i.lines.clear();
        assert_eq!(calculate(&i).unwrap().payable, "0.00");
        i.calculation_version = "unknown".into();
        assert!(calculate(&i).is_err());
    }
}

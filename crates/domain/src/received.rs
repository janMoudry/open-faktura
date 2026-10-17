use crate::{invoice::PartySnapshot, payment::PaymentInput};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Invoice,
    Receipt,
}
impl Kind {
    pub fn is_invoice(&self) -> bool {
        *self == Self::Invoice
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invoice => "invoice",
            Self::Receipt => "receipt",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock: Option<StockReceipt>,
    pub description: String,
    pub rate: String,
    pub net: String,
    pub vat: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock_incidental_cost: Option<String>,
    #[serde(default, skip_serializing_if = "Kind::is_invoice")]
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_method: Option<String>,
    pub supplier_contact_id: Option<String>,
    pub supplier: PartySnapshot,
    pub external_number: String,
    pub variable_symbol: String,
    pub bank_account: String,
    pub issued_on: NaiveDate,
    pub received_on: NaiveDate,
    pub due_on: NaiveDate,
    pub tax_on: Option<NaiveDate>,
    pub currency: String,
    pub description: String,
    pub note: String,
    pub lines: Vec<Line>,
    pub rounding: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Totals {
    pub net: String,
    pub vat: String,
    pub total: String,
}
pub fn money(v: Decimal) -> String {
    format!("{v:.2}")
}
pub fn amount(s: &str) -> Option<Decimal> {
    if s.is_empty()
        || s.len() > 16
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-')
    {
        return None;
    }
    let v = Decimal::from_str(s).ok()?;
    (v.scale() <= 2 && v.abs() <= Decimal::from_str("999999999999.99").unwrap()).then_some(v)
}
fn text(s: &str, max: usize) -> bool {
    s.chars().count() <= max && !s.chars().any(|c| c.is_control() && c != '\n')
}
pub fn totals(d: &Data) -> Result<Totals, Vec<String>> {
    let mut errors = vec![];
    if d.kind == Kind::Receipt {
        if !matches!(d.receipt_method.as_deref(), Some("cash" | "card"))
            || d.received_on != d.issued_on
            || d.due_on != d.issued_on
            || !d.variable_symbol.is_empty()
            || !d.bank_account.is_empty()
        {
            errors.push("receipt".into());
        }
    } else if d.receipt_method.is_some() {
        errors.push("receipt_method".into());
    }

    if [
        Some(d.issued_on),
        Some(d.received_on),
        Some(d.due_on),
        d.tax_on,
    ]
    .into_iter()
    .flatten()
    .any(|date| !(1900..=9999).contains(&date.year()))
    {
        errors.push("dates".into());
    }
    if d.currency != "CZK" {
        errors.push("currency".into())
    }
    if (d.kind == Kind::Invoice && d.supplier.name.trim().is_empty())
        || !text(&d.supplier.name, 200)
    {
        errors.push("supplier".into())
    }
    for s in [
        &d.supplier.registration_number,
        &d.supplier.vat_number,
        &d.supplier.email,
        &d.supplier.phone,
        &d.supplier.address.street,
        &d.supplier.address.city,
        &d.supplier.address.postal_code,
        &d.supplier.address.country,
    ] {
        if !text(s, 254) {
            errors.push("supplier".into())
        }
    }
    if (d.kind == Kind::Invoice && d.external_number.trim().is_empty())
        || !text(&d.external_number, 100)
    {
        errors.push("external_number".into())
    }
    if d.variable_symbol.len() > 10 || !d.variable_symbol.bytes().all(|b| b.is_ascii_digit()) {
        errors.push("variable_symbol".into())
    }
    if !text(&d.bank_account, 100) || !text(&d.description, 200) || !text(&d.note, 2000) {
        errors.push("text".into())
    }
    if d.due_on < d.issued_on {
        errors.push("due_on".into())
    }
    if d.lines.is_empty() || d.lines.len() > 100 {
        errors.push("lines".into())
    }
    let mut net = Decimal::ZERO;
    let mut vat = Decimal::ZERO;
    for l in &d.lines {
        if l.description.trim().is_empty() || !text(&l.description, 500) {
            errors.push("lines".into())
        }
        match (amount(&l.net), amount(&l.vat), amount(&l.rate)) {
            (Some(n), Some(v), Some(r))
                if n >= Decimal::ZERO
                    && v >= Decimal::ZERO
                    && r >= Decimal::ZERO
                    && r <= Decimal::from(100) =>
            {
                net += n;
                vat += v;
            }
            _ => errors.push("amount".into()),
        }
    }
    let rounding = amount(&d.rounding).filter(|r| r.abs() <= Decimal::ONE);
    if rounding.is_none() {
        errors.push("rounding".into())
    }
    let total = net + vat + rounding.unwrap_or_default();
    if total <= Decimal::ZERO || total > Decimal::from_str("999999999999.99").unwrap() {
        errors.push("total".into())
    }
    if errors.is_empty() {
        Ok(Totals {
            net: money(net),
            vat: money(vat),
            total: money(total),
        })
    } else {
        Err(errors)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    #[serde(flatten)]
    pub input: PaymentInput,
    pub actor_id: String,
    pub created_at: String,
    pub reversed_at: Option<String>,
    pub reversed_by: Option<String>,
    pub reason: Option<String>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_source_amounts() {
        assert_eq!(
            money(amount("100.01").unwrap() + amount("21.02").unwrap()),
            "121.03"
        );
        assert!(amount("0.001").is_none());
        assert!(amount("1e3").is_none());
        assert!(amount("NaN").is_none());
    }
}

/// Explicit physical receipt; financial lines alone never change stock.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StockReceipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_unit_cost: Option<String>,
    pub catalog_id: String,
    pub quantity: String,
    pub unit: String,
}

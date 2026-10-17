//! Stored tax amounts are deducted verbatim; never re-tax a received advance.
use crate::invoice::{Amounts, DraftData, PartySnapshot, TaxMode, TaxSummary, Totals};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxDeduction {
    pub id: String,
    pub number: String,
    pub totals: Totals,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub number: String,
    pub paid: String,
    pub payment_ids: Vec<String>,
    pub tax_receipts: Vec<TaxDeduction>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub version: String,
    pub sources: Vec<Source>,
    pub credited: String,
    pub rounding_adjustment: String,
    pub customer: PartySnapshot,
    pub supplier: PartySnapshot,
    pub tax_mode: TaxMode,
    pub minimum_date: chrono::NaiveDate,
}
fn d(v: &str) -> Decimal {
    Decimal::from_str(v).expect("persisted validated decimal")
}
fn money(mut v: Decimal) -> String {
    v.rescale(2);
    v.to_string()
}
pub fn same_party(a: &PartySnapshot, b: &PartySnapshot) -> bool {
    if !a.registration_number.trim().is_empty() && !b.registration_number.trim().is_empty() {
        a.registration_number.trim() == b.registration_number.trim()
            && a.address.country == b.address.country
    } else {
        a.name.trim() == b.name.trim() && a.address == b.address
    }
}
pub fn validate(data: &DraftData, s: &Settlement) -> Result<(), Vec<String>> {
    if data
        .customer
        .as_ref()
        .is_none_or(|c| !same_party(c, &s.customer))
        || !same_party(&data.supplier, &s.supplier)
        || data.calculation.tax_mode != s.tax_mode
        || data.calculation.currency != "CZK"
        || data.issued_on.is_none_or(|v| v < s.minimum_date)
        || (s.tax_mode == TaxMode::VatPayer && data.tax_on.is_none_or(|v| v < s.minimum_date))
    {
        return Err(vec!["advance_identity_or_date".into()]);
    }
    Ok(())
}
pub fn apply(mut total: Totals, s: &Settlement) -> Result<Totals, Vec<String>> {
    let mut groups: BTreeMap<String, (TaxMode, Decimal, Decimal)> = total
        .tax_summary
        .iter()
        .map(|g| {
            (
                g.vat_percent.clone(),
                (g.tax_mode, d(&g.amounts.net), d(&g.amounts.vat)),
            )
        })
        .collect();
    let (mut net, mut vat) = (d(&total.amounts.net), d(&total.amounts.vat));
    for src in &s.sources {
        for receipt in &src.tax_receipts {
            for row in &receipt.totals.tax_summary {
                let v = groups.entry(row.vat_percent.clone()).or_insert((
                    row.tax_mode,
                    Decimal::ZERO,
                    Decimal::ZERO,
                ));
                v.1 -= d(&row.amounts.net);
                v.2 -= d(&row.amounts.vat);
                net -= d(&row.amounts.net);
                vat -= d(&row.amounts.vat);
            }
        }
    }
    let payable = d(&total.payable) + d(&s.rounding_adjustment) - d(&s.credited);
    if payable < Decimal::ZERO {
        return Err(vec!["advance_exceeds_invoice".into()]);
    }
    total.amounts = Amounts {
        net: money(net),
        vat: money(vat),
        gross: money(net + vat),
    };
    total.rounding = money(d(&total.rounding) + d(&s.rounding_adjustment));
    total.payable = money(payable);
    total.tax_summary = groups
        .into_iter()
        .map(|(rate, (mode, n, v))| TaxSummary {
            vat_percent: rate,
            tax_mode: mode,
            amounts: Amounts {
                net: money(n),
                vat: money(v),
                gross: money(n + v),
            },
        })
        .collect();
    Ok(total)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::{calculate, Calculation, Line, PriceMode, Rounding};
    fn total(price: &str) -> Totals {
        calculate(&Calculation {
            calculation_version: "of-czk-v1".into(),
            currency: "CZK".into(),
            tax_mode: TaxMode::VatPayer,
            price_mode: PriceMode::Gross,
            discount_percent: "0".into(),
            rounding: Rounding::None,
            lines: vec![Line {
                catalog_id: None,
                id: "x".into(),
                description: "Example".into(),
                unit: "".into(),
                quantity: "1".into(),
                unit_price: price.into(),
                discount_percent: "0".into(),
                vat_percent: "21".into(),
            }],
        })
        .unwrap()
    }
    fn snapshot() -> Settlement {
        let party = PartySnapshot {
            name: "Test".into(),
            registration_number: "".into(),
            vat_number: "".into(),
            email: "".into(),
            phone: "".into(),
            address: crate::contact::Address {
                street: "A".into(),
                city: "B".into(),
                postal_code: "1".into(),
                country: "CZ".into(),
            },
        };
        Settlement {
            version: "of-settlement-v1".into(),
            sources: vec![],
            credited: "0.03".into(),
            rounding_adjustment: "0.00".into(),
            customer: party.clone(),
            supplier: party,
            tax_mode: TaxMode::VatPayer,
            minimum_date: "2026-10-12".parse().unwrap(),
        }
    }
    #[test]
    fn preserves_stored_penny_tax_breakdowns() {
        let mut s = snapshot();
        s.sources.push(Source {
            id: "1".into(),
            number: "ZF1".into(),
            paid: "0.03".into(),
            payment_ids: vec![],
            tax_receipts: (0..3)
                .map(|i| TaxDeduction {
                    id: i.to_string(),
                    number: format!("DDPP{i}"),
                    totals: total("0.01"),
                })
                .collect(),
        });
        // Three receipts each have net .01 and VAT zero; the final supply has net .02, VAT .01.
        let v = apply(total("0.03"), &s).unwrap();
        assert_eq!(v.amounts.net, "-0.01");
        assert_eq!(v.amounts.vat, "0.01");
        assert_eq!(v.payable, "0.00");
        assert!(apply(total("0.02"), &s).is_err());
        let mut cash = snapshot();
        cash.sources.clear();
        let v = apply(total("0.03"), &cash).unwrap();
        assert_eq!(v.amounts.net, "0.02");
        assert_eq!(v.amounts.vat, "0.01");
        assert_eq!(v.payable, "0.00");
    }
}

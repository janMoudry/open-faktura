//! Exact cumulative corrections of stored supply amounts; our own allocation rule.
use crate::invoice::{Amounts, DraftData, LineResult, TaxSummary, Totals};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Correction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_quantity: Option<String>,
    pub id: String,
    pub amount: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub reason: String,
    pub issued_on: chrono::NaiveDate,
    pub tax_on: chrono::NaiveDate,
    pub due_on: chrono::NaiveDate,
    pub language: String,
    pub lines: Vec<Correction>,
}
pub fn decimal(s: &str) -> Decimal {
    Decimal::from_str(s).expect("validated persisted decimal")
}
pub fn money(v: Decimal) -> String {
    let mut n = v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    n.rescale(2);
    n.to_string()
}
fn allocated(value: Decimal, part: Decimal, total: Decimal) -> Option<Decimal> {
    let cents = |mut v: Decimal| {
        v.rescale(2);
        v.mantissa()
    };
    let denominator = cents(total);
    let numerator = cents(value).checked_mul(cents(part))?;
    let rounded = numerator
        .abs()
        .checked_add(denominator / 2)?
        .checked_div(denominator)?
        * numerator.signum();
    Some(Decimal::from_i128_with_scale(rounded, 2))
}
pub fn capacities(t: &Totals) -> Vec<Correction> {
    t.lines
        .iter()
        .map(|l| Correction {
            return_quantity: None,
            id: l.id.clone(),
            amount: l.amounts.gross.clone(),
        })
        .collect()
}
/// Used amounts come from issued corrections only. Drafts never reserve value.
pub fn calculate(
    source: &DraftData,
    totals: &Totals,
    selected: &[Correction],
    used: &BTreeMap<String, Decimal>,
) -> Result<Totals, Vec<String>> {
    let bad = || vec!["credit_amount".into()];
    if totals.calculation_version != "of-czk-v1"
        || totals.currency != "CZK"
        || selected.is_empty()
        || selected.len() > 500
    {
        return Err(bad());
    }
    let caps = capacities(totals);
    let mut seen = HashSet::new();
    let mut lines = Vec::new();
    let mut groups: BTreeMap<String, (Decimal, Decimal)> = BTreeMap::new();
    let (mut net, mut vat, mut cash) = (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    for s in selected {
        if !seen.insert(&s.id) {
            return Err(bad());
        }
        let amount =
            crate::invoice::decimal(&s.amount, 2, 2_000_000_000_000_000).ok_or_else(bad)?;
        let index = caps.iter().position(|c| c.id == s.id).ok_or_else(bad)?;
        let cap = decimal(&caps[index].amount);
        let before = used.get(&s.id).copied().unwrap_or_default();
        if amount + before > cap || cap < Decimal::ZERO {
            return Err(bad());
        }
        if amount == Decimal::ZERO {
            continue;
        }
        let original = &totals.lines[index].amounts;
        let portion = |v: &str| -> Result<Decimal, Vec<String>> {
            Ok(allocated(decimal(v), before + amount, cap).ok_or_else(bad)?
                - allocated(decimal(v), before, cap).ok_or_else(bad)?)
        };
        let n = portion(&original.net)?;
        let v = portion(&original.vat)?;
        let rate = source
            .calculation
            .lines
            .iter()
            .find(|l| l.id == s.id)
            .ok_or_else(bad)?
            .vat_percent
            .clone();
        let group = groups.entry(rate).or_default();
        group.0 -= n;
        group.1 -= v;
        lines.push(LineResult {
            id: s.id.clone(),
            amounts: Amounts {
                net: money(-n),
                vat: money(-v),
                gross: money(-n - v),
            },
        });
        net -= n;
        vat -= v;
        cash -= amount;
    }
    // Allocate document rounding separately: a tiny last line must never get a negative capacity.
    let supply: Decimal = caps.iter().map(|c| decimal(&c.amount)).sum();
    let previous: Decimal = caps
        .iter()
        .map(|c| used.get(&c.id).copied().unwrap_or_default())
        .sum();
    let source_rounding = decimal(&totals.rounding);
    cash -= allocated(source_rounding, previous - cash, supply).ok_or_else(bad)?
        - allocated(source_rounding, previous, supply).ok_or_else(bad)?;
    if cash >= Decimal::ZERO {
        return Err(bad());
    }
    Ok(Totals {
        calculation_version: "of-credit-v1".into(),
        currency: "CZK".into(),
        lines,
        tax_summary: groups
            .into_iter()
            .map(|(rate, (n, v))| TaxSummary {
                tax_mode: source.calculation.tax_mode,
                vat_percent: rate,
                amounts: Amounts {
                    net: money(n),
                    vat: money(v),
                    gross: money(n + v),
                },
            })
            .collect(),
        amounts: Amounts {
            net: money(net),
            vat: money(vat),
            gross: money(net + vat),
        },
        rounding: money(cash - net - vat),
        payable: money(cash),
    })
}

//! Stored payment/tax allocation for sales receipts. Own rule, not verified iDoklad arithmetic.
use crate::invoice::{self, Amounts, DraftData, TaxSummary, Totals};
use crate::received::{amount, money};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payments {
    pub cash: String,
    pub card: String,
    pub voucher: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub invoice: DraftData,
    pub terminal_id: Option<String>,
    pub payments: Payments,
    pub internal_note: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Allocation {
    pub method: String,
    pub paid: String,
    pub tax_summary: Vec<TaxSummary>,
    pub rounding: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preview {
    pub totals: Totals,
    pub allocations: Vec<Allocation>,
}
pub fn preview(d: &Data) -> Result<Preview, Vec<String>> {
    let mut errors = invoice::validate_draft(&d.invoice);
    if d.invoice.calculation.currency != "CZK" {
        errors.push("currency".into());
    }
    if d.invoice.issued_on.is_none() {
        errors.push("issued_on".into());
    }
    if d.invoice.calculation.lines.is_empty() {
        errors.push("lines".into());
    }
    if !["cs", "en"].contains(&d.invoice.language.as_str()) {
        errors.push("language".into());
    }
    if d.internal_note.chars().count() > 2000
        || d.internal_note.chars().any(|c| c.is_control() && c != '\n')
    {
        errors.push("internal_note".into());
    }
    if d.invoice.supplier.address.country != "CZ"
        || d.invoice
            .customer
            .as_ref()
            .is_some_and(|p| p.address.country != "CZ")
    {
        errors.push("country".into());
    }
    let totals = invoice::calculate(&d.invoice.calculation)?;
    let payable = amount(&totals.payable).ok_or_else(|| vec!["amount".into()])?;
    if payable <= Decimal::ZERO {
        errors.push("amount".into());
    }
    if d.invoice.calculation.tax_mode == invoice::TaxMode::VatPayer
        && payable > Decimal::from(10000)
    {
        errors.push("retail_limit".into());
    }
    let methods = [
        ("cash", &d.payments.cash),
        ("card", &d.payments.card),
        ("voucher", &d.payments.voucher),
    ];
    let mut parts = vec![];
    for (method, value) in methods {
        match amount(value) {
            Some(v) if v >= Decimal::ZERO => parts.push((method, v)),
            _ => errors.push("payments".into()),
        }
    }
    if parts.iter().map(|(_, v)| v).sum::<Decimal>() != payable {
        errors.push("payments".into());
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut cumulative = Decimal::ZERO;
    let mut allocations = vec![];
    for (method, paid) in parts {
        if paid.is_zero() {
            continue;
        }
        let before = cumulative;
        cumulative += paid;
        let split = |value: &str| {
            let n = amount(value).expect("validated calculation");
            let at = |p: Decimal| {
                if p == payable {
                    n
                } else {
                    (n * p / payable)
                        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
                }
            };
            at(cumulative) - at(before)
        };
        let tax_summary: Vec<_> = totals
            .tax_summary
            .iter()
            .map(|t| {
                let net = split(&t.amounts.net);
                let vat = split(&t.amounts.vat);
                TaxSummary {
                    tax_mode: t.tax_mode,
                    vat_percent: t.vat_percent.clone(),
                    amounts: Amounts {
                        net: money(net),
                        vat: money(vat),
                        gross: money(net + vat),
                    },
                }
            })
            .collect();
        let gross: Decimal = tax_summary
            .iter()
            .map(|t| amount(&t.amounts.gross).unwrap())
            .sum();
        allocations.push(Allocation {
            method: method.into(),
            paid: money(paid),
            tax_summary,
            rounding: money(paid - gross),
        });
    }
    Ok(Preview {
        totals,
        allocations,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn data() -> Data {
        let mut invoice: DraftData =
            serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap();
        invoice.customer = None;
        invoice.calculation.lines[0].unit_price = "100".into();
        invoice.calculation.lines[0].quantity = "1".into();
        Data {
            invoice,
            terminal_id: None,
            payments: Payments {
                cash: "60.50".into(),
                card: "60.50".into(),
                voucher: "0".into(),
            },
            internal_note: String::new(),
        }
    }
    #[test]
    fn mixed_tender_has_exact_stored_tax() {
        let p = preview(&data()).unwrap();
        assert_eq!(p.allocations.len(), 2);
        for a in p.allocations {
            assert_eq!(a.tax_summary[0].amounts.net, "50.00");
            assert_eq!(a.tax_summary[0].amounts.vat, "10.50");
            assert_eq!(a.rounding, "0.00");
        }
    }
    #[test]
    fn cent_residual_and_overpayment() {
        let mut d = data();
        d.invoice.calculation.lines[0].unit_price = "0.03".into();
        d.invoice.calculation.price_mode = invoice::PriceMode::Gross;
        d.payments = Payments {
            cash: "0.01".into(),
            card: "0.01".into(),
            voucher: "0.01".into(),
        };
        let p = preview(&d).unwrap();
        assert_eq!(p.totals.amounts.net, "0.02");
        assert_eq!(p.totals.amounts.vat, "0.01");
        assert_eq!(p.allocations[0].tax_summary[0].amounts.net, "0.01");
        assert_eq!(p.allocations[1].tax_summary[0].amounts.vat, "0.01");
        assert_eq!(p.allocations[2].tax_summary[0].amounts.net, "0.01");
        d.payments.cash = "0.02".into();
        assert!(preview(&d).is_err());
        d.payments.cash = "-0.01".into();
        assert!(preview(&d).is_err());
    }
}

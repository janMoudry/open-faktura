//! Own documented allocation rule; not verified iDoklad numeric parity.
use crate::invoice::{Amounts, TaxMode, TaxSummary, Totals};
use rust_decimal::{Decimal, RoundingStrategy};
use std::str::FromStr;
pub const VERSION: &str = "of-tax-receipt-v1";
fn parse(s: &str) -> Result<Decimal, Vec<String>> {
    Decimal::from_str(s).map_err(|_| vec!["amount".into()])
}
fn money(mut d: Decimal) -> String {
    d.rescale(2);
    d.to_string()
}
pub fn allocate(source: &Totals, amount: &str) -> Result<Vec<TaxSummary>, Vec<String>> {
    let payment = crate::invoice::decimal(amount, 2, 999_999_999_999)
        .filter(|v| *v > Decimal::ZERO)
        .ok_or_else(|| vec!["amount".into()])?;
    let mut groups = Vec::new();
    for row in &source.tax_summary {
        if row.tax_mode != TaxMode::VatPayer
            || !["0", "12", "21"].contains(&row.vat_percent.as_str())
        {
            return Err(vec!["tax_mode".into()]);
        }
        let gross = parse(&row.amounts.gross)?;
        if gross > Decimal::ZERO {
            groups.push((parse(&row.vat_percent)?, gross));
        }
    }
    groups.sort_by_key(|g| g.0);
    let total: Decimal = groups.iter().map(|g| g.1).sum();
    if total <= Decimal::ZERO || payment > parse(&source.payable)? {
        return Err(vec!["amount".into()]);
    }
    let mut values: Vec<_> = groups
        .iter()
        .map(|(_, gross)| {
            // Divide before multiplying to stay within Decimal's intermediate range.
            let exact = payment * (gross / total);
            let rounded = exact.round_dp_with_strategy(2, RoundingStrategy::ToZero);
            (rounded, exact - rounded)
        })
        .collect();
    let mut order: Vec<usize> = (0..groups.len()).collect();
    order.sort_by(|a, b| values[*b].1.cmp(&values[*a].1).then(a.cmp(b)));
    let mut remaining = payment - values.iter().map(|v| v.0).sum::<Decimal>();
    for i in order {
        if remaining <= Decimal::ZERO {
            break;
        }
        values[i].0 += Decimal::new(1, 2);
        remaining -= Decimal::new(1, 2);
    }
    Ok(groups
        .into_iter()
        .zip(values)
        .filter(|(_, v)| v.0 > Decimal::ZERO)
        .map(|((rate, _), (gross, _))| {
            let net = (gross / (Decimal::ONE + rate / Decimal::from(100)))
                .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
            TaxSummary {
                tax_mode: TaxMode::VatPayer,
                vat_percent: rate.normalize().to_string(),
                amounts: Amounts {
                    net: money(net),
                    vat: money(gross - net),
                    gross: money(gross),
                },
            }
        })
        .collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Totals {
        Totals {
            calculation_version: "of-czk-v1".into(),
            currency: "CZK".into(),
            lines: vec![],
            tax_summary: vec![
                TaxSummary {
                    tax_mode: TaxMode::VatPayer,
                    vat_percent: "12".into(),
                    amounts: Amounts {
                        net: "1000".into(),
                        vat: "120".into(),
                        gross: "1120".into(),
                    },
                },
                TaxSummary {
                    tax_mode: TaxMode::VatPayer,
                    vat_percent: "21".into(),
                    amounts: Amounts {
                        net: "1000".into(),
                        vat: "210".into(),
                        gross: "1210".into(),
                    },
                },
            ],
            amounts: Amounts {
                net: "2000".into(),
                vat: "330".into(),
                gross: "2330".into(),
            },
            rounding: "0".into(),
            payable: "2330".into(),
        }
    }
    #[test]
    fn half_payment_and_penny_remainder() {
        let v = allocate(&source(), "1165").unwrap();
        assert_eq!(
            v[0].amounts,
            Amounts {
                net: "500.00".into(),
                vat: "60.00".into(),
                gross: "560.00".into()
            }
        );
        assert_eq!(
            v[1].amounts,
            Amounts {
                net: "500.00".into(),
                vat: "105.00".into(),
                gross: "605.00".into()
            }
        );
        let p = allocate(&source(), "0.01").unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].vat_percent, "21");
        assert_eq!(p[0].amounts.gross, "0.01");
        assert!(allocate(&source(), "2330.01").is_err());
        assert!(allocate(&source(), "0").is_err());
        assert!(allocate(&source(), "1.001").is_err());
    }
}

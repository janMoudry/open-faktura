//! Historical import validation: supplied totals are checked, never recomputed.
use crate::{
    contact::ContactData,
    invoice::{self, DraftData, Totals},
    payment::PaymentInput,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Contact {
        key: String,
        data: Box<ContactData>,
    },
    Invoice {
        key: String,
        number: String,
        contact_key: Option<String>,
        data: Box<DraftData>,
        totals: Box<Totals>,
        payments: Vec<PaymentInput>,
    },
}
impl Record {
    pub fn key(&self) -> &str {
        match self {
            Self::Contact { key, .. } | Self::Invoice { key, .. } => key,
        }
    }
    pub fn kind(&self) -> &str {
        match self {
            Self::Contact { .. } => "contact",
            Self::Invoice { .. } => "invoice",
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Contact { data, .. } => &data.name,
            Self::Invoice { number, .. } => number,
        }
    }
    pub fn validate(&mut self) -> Vec<String> {
        let mut errors = vec![];
        if self.key().trim().is_empty()
            || self.key().len() > 200
            || self.key().chars().any(char::is_control)
        {
            errors.push("source_key".into());
        }
        match self {
            Self::Contact { data, .. } => errors.extend(data.normalize_validate()),
            Self::Invoice {
                number,
                contact_key,
                data,
                totals,
                payments,
                ..
            } => {
                if number.trim().is_empty()
                    || number.len() > 100
                    || number.chars().any(char::is_control)
                {
                    errors.push("number".into());
                }
                if contact_key
                    .as_ref()
                    .is_some_and(|k| k.is_empty() || k.len() > 200)
                {
                    errors.push("contact_key".into());
                }
                // Reuse shape/range checks, discard calculated totals. No source totals are replaced.
                let mut shape = *data.clone();
                shape.calculation.calculation_version = invoice::VERSION.into();
                errors.extend(invoice::validate_issue(&shape).err().unwrap_or_default());
                if data.contact_id.is_some() {
                    errors.push("contact_id".into());
                }
                if ![invoice::VERSION, "import-v1"]
                    .contains(&data.calculation.calculation_version.as_str())
                {
                    errors.push("calculation_version".into());
                }
                if totals.currency != data.calculation.currency {
                    errors.push("currency".into());
                }
                let mut sum = [Decimal::ZERO; 3];
                let amount = |a: &invoice::Amounts| -> Option<[Decimal; 3]> {
                    if [&a.net, &a.vat, &a.gross]
                        .iter()
                        .any(|v| v.rsplit_once('.').is_none_or(|(_, f)| f.len() != 2))
                    {
                        return None;
                    }
                    let n = money(&a.net)?;
                    let v = money(&a.vat)?;
                    let g = money(&a.gross)?;
                    (n + v == g).then_some([n, v, g])
                };
                if totals.lines.len() != data.calculation.lines.len() {
                    errors.push("totals.lines".into());
                }
                let mut buckets: std::collections::BTreeMap<String, [Decimal; 3]> =
                    Default::default();
                for (line, source) in totals.lines.iter().zip(&data.calculation.lines) {
                    if line.id != source.id {
                        errors.push("totals.lines".into());
                    }
                    if let Some(a) = amount(&line.amounts) {
                        let bucket = buckets.entry(source.vat_percent.clone()).or_default();
                        for i in 0..3 {
                            sum[i] += a[i];
                            bucket[i] += a[i];
                        }
                        if source.vat_percent == "0" && a[1] != Decimal::ZERO {
                            errors.push("totals.lines".into());
                        }
                    } else {
                        errors.push("totals.lines".into());
                    }
                }
                if amount(&totals.amounts) != Some(sum) {
                    errors.push("totals.amounts".into());
                }
                let mut tax_sum = [Decimal::ZERO; 3];
                for tax in &totals.tax_summary {
                    if !["0", "12", "21"].contains(&tax.vat_percent.as_str())
                        || tax.tax_mode != data.calculation.tax_mode
                    {
                        errors.push("totals.tax_summary".into());
                    }
                    if let Some(a) = amount(&tax.amounts) {
                        if buckets.remove(&tax.vat_percent) != Some(a) {
                            errors.push("totals.tax_summary".into());
                        }
                        for i in 0..3 {
                            tax_sum[i] += a[i];
                        }
                    } else {
                        errors.push("totals.tax_summary".into());
                    }
                }
                if tax_sum != sum || !buckets.is_empty() {
                    errors.push("totals.tax_summary".into());
                }
                if [&totals.rounding, &totals.payable]
                    .iter()
                    .any(|v| v.rsplit_once('.').is_none_or(|(_, f)| f.len() != 2))
                {
                    errors.push("totals.amounts".into());
                }
                let rounding = signed_money(&totals.rounding);
                let payable = money(&totals.payable);
                if rounding.is_none_or(|r| r.abs() > Decimal::ONE)
                    || payable.is_none()
                    || rounding.zip(payable).is_none_or(|(r, p)| sum[2] + r != p)
                {
                    errors.push("totals.payable".into());
                }
                if payments.len() > 100 {
                    errors.push("payments".into());
                }
                for payment in payments.iter() {
                    errors.extend(payment.validate());
                    if Some(payment.paid_on) < data.issued_on || payable == Some(Decimal::ZERO) {
                        errors.push("payments".into());
                    }
                }
                data.calculation.calculation_version = "import-v1".into();
                totals.calculation_version = "import-v1".into();
            }
        }
        errors.sort();
        errors.dedup();
        errors
    }
}
pub fn money(s: &str) -> Option<Decimal> {
    signed_money(s).filter(|v| *v >= Decimal::ZERO && !s.starts_with('-'))
}
fn signed_money(s: &str) -> Option<Decimal> {
    let raw = s.strip_prefix('-').unwrap_or(s);
    let parts: Vec<_> = raw.split('.').collect();
    if raw.len() > 15
        || parts.len() > 2
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        || parts.get(1).is_some_and(|p| p.len() > 2)
    {
        return None;
    }
    Decimal::from_str(s).ok()
}

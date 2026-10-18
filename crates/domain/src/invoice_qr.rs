//! QR Faktura SID 1.0, optionally nested in SPAYD X-INV. Immutable domestic snapshot.
use crate::invoice::{DraftData, QrFormat, TaxMode, Totals};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
fn encode(v: &str) -> String {
    v.as_bytes()
        .iter()
        .map(|&b| {
            if b.is_ascii_alphanumeric() || b"-._~:/+ ".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn sid(fields: &BTreeMap<&str, String>) -> String {
    let mut result = String::from("SID*1.0*");
    for (k, v) in fields {
        result.push_str(&format!("{k}:{}*", encode(v)));
    }
    result
}
pub fn payload(data: &DraftData, totals: &Totals, number: &str) -> Option<String> {
    if data.qr_format == QrFormat::Payment {
        return crate::payment_qr::payload(
            &data.payment,
            &totals.payable,
            &totals.currency,
            data.due_on,
        );
    }
    if totals.currency != "CZK"
        || number.is_empty()
        || number.chars().count() > 40
        || number.chars().any(char::is_control)
        || data.issued_on? < "2024-01-01".parse().ok()?
    {
        return None;
    }
    let payable: Decimal = totals.payable.parse().ok()?;
    if payable <= Decimal::ZERO
        || totals.payable.len() > 18
        || !totals
            .payable
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.')
    {
        return None;
    }
    let mut fields = BTreeMap::new();
    fields.insert("ID", number.to_string());
    fields.insert("DD", data.issued_on?.format("%Y%m%d").to_string());
    fields.insert("AM", totals.payable.clone());
    fields.insert(
        "TD",
        if data.calculation.tax_mode == TaxMode::NonPayer {
            "0"
        } else {
            "9"
        }
        .into(),
    );
    if let Some(date) = data.tax_on {
        fields.insert("DUZP", date.format("%Y%m%d").to_string());
    }
    if let Some(date) = data.due_on {
        fields.insert("DT", date.format("%Y%m%d").to_string());
    }
    let mut buckets: BTreeMap<&str, (Decimal, Decimal)> = BTreeMap::new();
    for tax in &totals.tax_summary {
        let rate: Decimal = tax.vat_percent.parse().ok()?;
        let key = if rate == Decimal::from(21) {
            "0"
        } else if rate == Decimal::from(12) {
            "1"
        } else if rate.is_zero() {
            "N"
        } else {
            return None;
        };
        let net: Decimal = tax.amounts.net.parse().ok()?;
        let vat: Decimal = tax.amounts.vat.parse().ok()?;
        let entry = buckets.entry(key).or_default();
        entry.0 = entry.0.checked_add(net)?;
        entry.1 = entry.1.checked_add(vat)?;
    }
    if buckets.is_empty() {
        return None;
    }
    for (key, (net, vat)) in buckets {
        match key {
            "0" => {
                fields.insert("TB0", format!("{net:.2}"));
                fields.insert("T0", format!("{vat:.2}"));
            }
            "1" => {
                fields.insert("TB1", format!("{net:.2}"));
                fields.insert("T1", format!("{vat:.2}"));
            }
            _ => {
                if !vat.is_zero() {
                    return None;
                }
                fields.insert("NTB", format!("{net:.2}"));
            }
        }
    }
    if data.supplier.registration_number.len() <= 8
        && !data.supplier.registration_number.is_empty()
        && data
            .supplier
            .registration_number
            .bytes()
            .all(|b| b.is_ascii_digit())
    {
        fields.insert("INI", data.supplier.registration_number.clone());
    }
    if data.supplier.vat_number.len() <= 14
        && !data.supplier.vat_number.is_empty()
        && data
            .supplier
            .vat_number
            .bytes()
            .all(|b| b.is_ascii_alphanumeric())
    {
        fields.insert("VII", data.supplier.vat_number.clone());
    }
    let vs = &data.payment.variable_symbol;
    if vs.len() <= 10 && !vs.is_empty() && vs.bytes().all(|b| b.is_ascii_digit()) {
        fields.insert("VS", vs.clone());
    }
    if let Some(iban) = crate::payment_qr::iban(&data.payment) {
        fields.insert("ACC", iban);
    }
    if data.qr_format == QrFormat::PaymentInvoice {
        let payment = crate::payment_qr::payload(
            &data.payment,
            &totals.payable,
            &totals.currency,
            data.due_on,
        )?;
        for key in ["AM", "CC", "DT", "ACC", "VS"] {
            fields.remove(key);
        }
        // Nested descriptor separators and percent escapes must be encoded once
        // more so the outer SPD parser cannot treat them as payment attributes.
        let nested = sid(&fields)
            .bytes()
            .map(|b| {
                if b == b'*' || b == b'%' {
                    format!("%{b:02X}")
                } else {
                    (b as char).to_string()
                }
            })
            .collect::<String>();
        Some(format!("{payment}*X-INV:{nested}"))
    } else {
        Some(sid(&fields))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_is_omitted_for_legacy_replay_and_combined_is_escaped() {
        let mut d: DraftData =
            serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap();
        assert!(serde_json::to_value(&d).unwrap().get("qr_format").is_none());
        d.payment.method = "bank_transfer".into();
        d.payment.iban = "CZ6508000000192000145399".into();
        d.qr_format = QrFormat::Invoice;
        let totals = crate::invoice::calculate(&d.calculation).unwrap();
        let plain = payload(&d, &totals, "SYNTH*001").unwrap();
        assert!(plain.starts_with("SID*1.0*"));
        assert!(plain.contains("ID:SYNTH%2A001*"));
        assert!(plain.contains("TB0:59.99*"));
        d.qr_format = QrFormat::PaymentInvoice;
        let combined = payload(&d, &totals, "SYNTH*001").unwrap();
        assert!(combined.starts_with("SPD*1.0*ACC:CZ6508000000192000145399"));
        assert!(combined.contains("X-INV:SID%2A1.0%2A"));
        assert!(combined.contains("ID:SYNTH%252A001"));
        d.payment.iban = "DE89370400440532013000".into();
        assert!(payload(&d, &totals, "SYNTH").is_none());
    }
}

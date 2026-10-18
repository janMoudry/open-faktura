//! SPAYD 1.0 for immutable CZK document snapshots. No external I/O.
use crate::{company::valid_iban, invoice::PaymentSnapshot};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;
fn account_part(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.bytes().all(|b| b.is_ascii_digit())
        && s.bytes()
            .rev()
            .enumerate()
            .map(|(i, b)| u32::from(b - b'0') * ((1u32 << i) % 11))
            .sum::<u32>()
            % 11
            == 0
}
pub fn iban(payment: &PaymentSnapshot) -> Option<String> {
    let supplied = payment
        .iban
        .split_whitespace()
        .collect::<String>()
        .to_uppercase();
    if !supplied.is_empty() {
        return valid_iban(&supplied).then_some(supplied);
    }
    let raw = payment.bank_account.trim();
    let (account, bank) = raw.split_once('/')?;
    if bank.len() != 4 || !bank.bytes().all(|b| b.is_ascii_digit()) || bank == "0000" {
        return None;
    }
    let (prefix, number) = account.split_once('-').unwrap_or(("0", account));
    if !account_part(prefix, 6) || !account_part(number, 10) || number.bytes().all(|b| b == b'0') {
        return None;
    }
    let bban = format!("{bank}{prefix:0>6}{number:0>10}");
    let rem = format!("{bban}123500")
        .bytes()
        .fold(0u32, |r, b| (r * 10 + u32::from(b - b'0')) % 97);
    Some(format!("CZ{:02}{bban}", 98 - rem))
}
pub fn payload(
    payment: &PaymentSnapshot,
    amount: &str,
    currency: &str,
    due: Option<NaiveDate>,
) -> Option<String> {
    if payment.method != "bank_transfer" || currency != "CZK" {
        return None;
    }
    // Reject ambiguous exponents, excess scale, signs and non-finite values before Decimal parsing.
    if amount.is_empty()
        || amount.len() > 10
        || !amount.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        || amount
            .split_once('.')
            .is_some_and(|(_, v)| v.is_empty() || v.len() > 2)
    {
        return None;
    }
    let mut value = Decimal::from_str(amount).ok()?;
    if value <= Decimal::ZERO || value > Decimal::new(999999999, 2) {
        return None;
    }
    let account = iban(payment)?;
    let vs = &payment.variable_symbol;
    if vs.len() > 10 || !vs.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.rescale(2);
    let mut s = format!("SPD*1.0*ACC:{account}*AM:{value}*CC:CZK");
    if let Some(d) = due {
        s.push_str(&format!("*DT:{}", d.format("%Y%m%d")));
    }
    if !vs.is_empty() {
        s.push_str(&format!("*X-VS:{vs}"));
    }
    Some(s)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn payment() -> PaymentSnapshot {
        PaymentSnapshot {
            method: "bank_transfer".into(),
            bank_account: "19-2000145399/0800".into(),
            iban: "".into(),
            variable_symbol: "001234".into(),
        }
    }
    #[test]
    fn documented_account_and_exact_payload() {
        let p = payment();
        assert_eq!(iban(&p).as_deref(), Some("CZ6508000000192000145399"));
        assert_eq!(
            payload(&p, "1234.50", "CZK", Some("2026-10-20".parse().unwrap())).unwrap(),
            "SPD*1.0*ACC:CZ6508000000192000145399*AM:1234.50*CC:CZK*DT:20261020*X-VS:001234"
        );
    }
    #[test]
    fn invalid_payment_never_becomes_a_different_payment() {
        let mut p = payment();
        for v in ["0", "-1", "1.001", "10000000.00", "1e3", "NaN", "1..0"] {
            assert!(payload(&p, v, "CZK", None).is_none());
        }
        assert!(payload(&p, "9999999.99", "CZK", None).is_some());
        assert!(payload(&p, "1", "EUR", None).is_none());
        p.iban = "CZ6508000000192000145390".into();
        assert!(payload(&p, "1", "CZK", None).is_none());
        p.iban.clear();
        p.variable_symbol = "1*AM:99".into();
        assert!(payload(&p, "1", "CZK", None).is_none());
        p.variable_symbol.clear();
        p.bank_account = "19-2000145390/0800".into();
        assert!(payload(&p, "1", "CZK", None).is_none());
        p = payment();
        p.method = "cash".into();
        assert!(payload(&p, "1", "CZK", None).is_none());
    }
}

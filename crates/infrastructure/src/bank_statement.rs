//! Bank transport adapters. Fixtures are synthetic contract examples, not real exports.
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::NaiveDate;
use faktura_application::banking::{Error, MovementData, Result, StatementInput};
use faktura_domain::received::{amount, money};
use quick_xml::{events::Event, Reader};
use std::collections::BTreeMap;
fn invalid(field: &str) -> Error {
    Error::Invalid(vec![field.into()])
}
fn bad() -> Error {
    invalid("statement_format")
}
#[derive(Default)]
pub(crate) struct Node {
    pub(crate) name: String,
    text: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
}
impl Node {
    pub(crate) fn at(&self, path: &str) -> Option<&Node> {
        let mut n = self;
        for part in path.split('/') {
            n = n.children.iter().find(|c| c.name == part)?;
        }
        Some(n)
    }
    pub(crate) fn value(&self, path: &str) -> &str {
        self.at(path).map(|n| n.text.trim()).unwrap_or("")
    }
    pub(crate) fn attr(&self, name: &str) -> &str {
        self.attrs.get(name).map(String::as_str).unwrap_or("")
    }
    pub(crate) fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |n| n.name == name)
    }
}
pub(crate) fn xml(s: &str) -> Result<Node> {
    let mut reader = Reader::from_str(s);
    let mut stack = vec![Node::default()];
    let mut count = 0;
    loop {
        match reader.read_event().map_err(|_| bad())? {
            Event::Start(e) | Event::Empty(e) => {
                // Empty elements are detected from the consumed source, not expanded externally.
                let empty = s.as_bytes().get(reader.buffer_position() as usize - 2) == Some(&b'/');
                count += 1;
                if count > 60000 || stack.len() > 32 {
                    return Err(bad());
                }
                let mut n = Node {
                    name: String::from_utf8(e.local_name().as_ref().to_vec()).map_err(|_| bad())?,
                    ..Default::default()
                };
                for a in e.attributes() {
                    let a = a.map_err(|_| bad())?;
                    let key = String::from_utf8(a.key.as_ref().to_vec()).map_err(|_| bad())?;
                    n.attrs.insert(
                        key,
                        a.decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|_| bad())?
                        .into_owned(),
                    );
                }
                if empty {
                    stack.last_mut().ok_or_else(bad)?.children.push(n);
                } else {
                    stack.push(n);
                }
            }
            Event::End(_) => {
                if stack.len() < 2 {
                    return Err(bad());
                }
                let n = stack.pop().ok_or_else(bad)?;
                stack.last_mut().ok_or_else(bad)?.children.push(n);
            }
            Event::Text(e) => {
                stack
                    .last_mut()
                    .ok_or_else(bad)?
                    .text
                    .push_str(&e.decode().map_err(|_| bad())?);
            }
            Event::CData(e) => {
                stack
                    .last_mut()
                    .ok_or_else(bad)?
                    .text
                    .push_str(&e.decode().map_err(|_| bad())?);
            }
            Event::GeneralRef(e) => {
                let name = e.decode().map_err(|_| bad())?;
                let entity = format!("&{name};");
                let value = quick_xml::escape::unescape(&entity).map_err(|_| bad())?;
                stack.last_mut().ok_or_else(bad)?.text.push_str(&value);
            }
            Event::DocType(e) => {
                // A MONETA external DTD declaration is inert. Internal subsets/entities are rejected.
                if e.contains(&b'[') {
                    return Err(bad());
                }
            }
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) => {}
        }
    }
    if stack.len() != 1 {
        return Err(bad());
    }
    let mut root = stack.pop().ok_or_else(bad)?;
    if root.children.len() != 1 || !root.text.trim().is_empty() {
        return Err(bad());
    }
    Ok(root.children.remove(0))
}
fn date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| invalid("booked_on"))
}
fn account(actual: &str, expected: &str) -> Result<()> {
    if actual.split_whitespace().collect::<String>().to_uppercase() != expected {
        return Err(invalid("statement_account"));
    }
    Ok(())
}
fn row(
    id: &str,
    booked_on: NaiveDate,
    value: &str,
    currency: &str,
    vs: &str,
    party: &str,
    description: &str,
) -> Result<MovementData> {
    if id.is_empty() || ["NOTPROVIDED", "NONREF", "0"].contains(&id) {
        return Err(invalid("statement_id"));
    }
    if amount(value).is_none() {
        return Err(invalid("amount"));
    }
    let mut d = MovementData {
        external_id: id.into(),
        booked_on,
        amount: value.into(),
        currency: currency.into(),
        variable_symbol: vs.into(),
        counterparty: party.split_whitespace().collect::<Vec<_>>().join(" "),
        description: description.split_whitespace().collect::<Vec<_>>().join(" "),
    };
    d.normalize();
    let fields = d.validate();
    if !fields.is_empty() {
        return Err(Error::Invalid(fields));
    }
    Ok(d)
}
fn symbol(reference: &str) -> Result<String> {
    if let Some((_, rest)) = reference.split_once("/VS") {
        let value = rest.split('/').next().unwrap_or_default();
        if value.len() > 10 || value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid("variable_symbol"));
        }
        return Ok(value.into());
    }
    Ok(String::new())
}
fn camt(s: &str, iban: &str, currency: &str) -> Result<Vec<MovementData>> {
    let root = xml(s)?;
    if root.name != "Document" {
        return Err(bad());
    }
    let bank = root.at("BkToCstmrStmt").ok_or_else(bad)?;
    let mut rows = vec![];
    for statement in bank.all("Stmt") {
        account(statement.value("Acct/Id/IBAN"), iban)?;
        if statement.value("Acct/Ccy") != currency {
            return Err(invalid("currency"));
        }
        for entry in statement.all("Ntry") {
            let status = entry.value("Sts");
            if status != "BOOK" && entry.value("Sts/Cd") != "BOOK" {
                return Err(invalid("statement_status"));
            }
            for details in entry.all("NtryDtls") {
                let count = details.value("Btch/NbOfTxs");
                if !count.is_empty() && count.parse::<usize>().ok() != Some(1) {
                    return Err(invalid("statement_batch"));
                }
            }
            let transactions: Vec<_> = entry
                .all("NtryDtls")
                .flat_map(|n| n.all("TxDtls"))
                .collect();
            // Aggregated booking is never silently treated as one customer payment.
            if transactions.len() > 1 {
                return Err(invalid("statement_batch"));
            }
            let tx = transactions.first().copied();
            let reference = tx
                .map(|t| t.value("Refs/AcctSvcrRef"))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| entry.value("AcctSvcrRef"));
            let amt = entry.at("Amt").ok_or_else(bad)?;
            if amt.attr("Ccy") != statement.value("Acct/Ccy") {
                return Err(invalid("currency"));
            }
            let value = amount(amt.text.trim())
                .filter(|v| *v > rust_decimal::Decimal::ZERO)
                .ok_or_else(|| invalid("amount"))?;
            let incoming = match entry.value("CdtDbtInd") {
                "CRDT" => true,
                "DBIT" => false,
                _ => return Err(bad()),
            };
            let booking = entry.value("BookgDt/Dt");
            let booked = if booking.is_empty() {
                chrono::DateTime::parse_from_rfc3339(entry.value("BookgDt/DtTm"))
                    .map_err(|_| invalid("booked_on"))?
                    .date_naive()
            } else {
                date(booking)?
            };
            let mut vs = String::new();
            let mut party = "";
            let mut description = String::new();
            if let Some(tx) = tx {
                vs = symbol(tx.value("Refs/EndToEndId"))?;
                let other = symbol(tx.value("RmtInf/Strd/CdtrRefInf/Ref"))?;
                if !other.is_empty() {
                    if !vs.is_empty() && vs != other {
                        return Err(invalid("variable_symbol"));
                    }
                    vs = other;
                }
                party = tx.value(if incoming {
                    "RltdPties/Dbtr/Nm"
                } else {
                    "RltdPties/Cdtr/Nm"
                });
                if let Some(remit) = tx.at("RmtInf") {
                    description = remit
                        .all("Ustrd")
                        .map(|n| n.text.trim())
                        .collect::<Vec<_>>()
                        .join(" ");
                }
                if description.is_empty() {
                    description = tx.value("AddtlTxInf").into();
                }
            }
            if description.is_empty() {
                description = entry.value("AddtlNtryInf").into();
            }
            rows.push(row(
                reference,
                booked,
                &money(if incoming { value } else { -value }),
                amt.attr("Ccy"),
                &vs,
                party,
                &description,
            )?);
        }
    }
    Ok(rows)
}
fn json_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
fn fio(s: &str, iban: &str, currency: &str) -> Result<Vec<MovementData>> {
    let root: serde_json::Value = serde_json::from_str(s).map_err(|_| bad())?;
    let statement = &root["accountStatement"];
    account(statement["info"]["iban"].as_str().unwrap_or_default(), iban)?;
    if statement["info"]["currency"].as_str() != Some(currency) {
        return Err(invalid("currency"));
    }
    if statement.get("transactionList").is_none() {
        return Err(bad());
    }
    if statement["transactionList"].is_null()
        || statement["transactionList"]
            .get("transaction")
            .is_some_and(serde_json::Value::is_null)
    {
        return Ok(vec![]);
    }
    let transactions = statement["transactionList"]["transaction"]
        .as_array()
        .ok_or_else(bad)?;
    transactions
        .iter()
        .map(|t| {
            let value = |n| json_text(&t[format!("column{n}")]["value"]);
            if value(14) != statement["info"]["currency"].as_str().unwrap_or_default() {
                return Err(invalid("currency"));
            }
            let raw = value(0);
            let booked = if let Ok(epoch) = raw.parse::<i64>() {
                chrono::DateTime::from_timestamp_millis(epoch)
                    .ok_or_else(|| invalid("booked_on"))?
                    .with_timezone(&chrono_tz::Europe::Prague)
                    .date_naive()
            } else if raw.len() >= 10 {
                date(raw.get(..10).ok_or_else(bad)?)?
            } else {
                return Err(invalid("booked_on"));
            };
            let desc = [value(16), value(25)]
                .into_iter()
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            row(
                &value(22),
                booked,
                &value(1),
                &value(14),
                &value(5),
                &value(10),
                &desc,
            )
        })
        .collect()
}
fn moneta(s: &str, iban: &str, currency: &str) -> Result<Vec<MovementData>> {
    let root = xml(s)?;
    if root.name != "statement" {
        return Err(bad());
    }
    let header = root.at("header/account").ok_or_else(bad)?;
    account(header.attr("iban"), iban)?;
    let transactions = root.at("transactions").ok_or_else(bad)?;
    if header.attr("currency") != currency {
        return Err(invalid("currency"));
    }
    let mut rows = vec![];
    for t in transactions.all("transaction") {
        let raw = t.attr("date-post");
        let booked = date(raw).or_else(|_| {
            NaiveDate::parse_from_str(raw, "%d.%m.%Y").map_err(|_| invalid("booked_on"))
        })?;
        let value = t.attr("amount");
        let mut direction = None;
        let mut descriptions = vec![];
        for group in t.all("trn-messages") {
            let next = match group.attr("type") {
                "credit" => Some(true),
                "debit" => Some(false),
                _ => None,
            };
            if let Some(next) = next {
                if direction.is_some_and(|v| v != next) {
                    return Err(bad());
                }
                direction = Some(next);
            }
            for msg in group.all("trn-message") {
                descriptions.push(msg.text.trim());
            }
        }
        // DTD does not define signed amount semantics. Require an explicit debit/credit marker.
        let incoming = direction.ok_or_else(|| invalid("statement_direction"))?;
        let amount = amount(value).ok_or_else(|| invalid("amount"))?;
        if incoming && amount.is_sign_negative() {
            return Err(invalid("statement_direction"));
        }
        let amount = amount.abs();
        rows.push(row(
            t.attr("id"),
            booked,
            &money(if incoming { amount } else { -amount }),
            header.attr("currency"),
            t.attr("var-sym"),
            t.attr("other-account-number"),
            &descriptions.join(" "),
        )?);
    }
    if let Some(count) = root.at("header/stmt") {
        if count.attr("trn-cnt").parse::<usize>().ok() != Some(rows.len()) {
            return Err(bad());
        }
    }
    Ok(rows)
}
fn rb(s: &str, iban: &str, filename: &str) -> Result<Vec<MovementData>> {
    if iban.len() != 24 || !iban.starts_with("CZ") || &iban[4..8] != "5500" {
        return Err(invalid("statement_account"));
    }
    // ABO does not carry ISO currency; RB defines it in the exported filename.
    if !filename.to_ascii_uppercase().ends_with("_CZK.GPC") {
        return Err(invalid("statement_filename"));
    }
    let mut rows: Vec<MovementData> = vec![];
    let mut header = false;
    for line in s.lines() {
        if line.is_empty() {
            continue;
        }
        let chars: Vec<_> = line.chars().collect();
        let field = |a: usize, b: usize| -> Result<String> {
            Ok(chars.get(a..b).ok_or_else(bad)?.iter().collect())
        };
        match field(0, 3)?.as_str() {
            "074" => {
                if chars.len() != 128 {
                    return Err(bad());
                }
                account(&field(3, 19)?, &iban[8..])?;
                header = true;
            }
            "075" => {
                if !header || chars.len() != 128 {
                    return Err(bad());
                }
                account(&field(3, 19)?, &iban[8..])?;
                if field(117, 118)? != "0" {
                    return Err(invalid("statement_changed"));
                }
                let raw = field(48, 60)?;
                if !raw.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(invalid("amount"));
                }
                let cents = raw.parse::<i64>().map_err(|_| invalid("amount"))?;
                let signed = match field(60, 61)?.as_str() {
                    "1" | "5" => -cents,
                    "2" | "4" => cents,
                    _ => return Err(bad()),
                };
                let raw = field(122, 128)?;
                if !raw.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad());
                }
                let booked =
                    NaiveDate::parse_from_str(&format!("{}20{}", &raw[..4], &raw[4..]), "%d%m%Y")
                        .map_err(|_| invalid("booked_on"))?;
                rows.push(row(
                    field(35, 48)?.trim(),
                    booked,
                    &money(rust_decimal::Decimal::new(signed, 2)),
                    "CZK",
                    field(61, 71)?.trim(),
                    field(97, 117)?.trim(),
                    "",
                )?);
            }
            "078" | "079" => {
                if chars.len() != 73 {
                    return Err(bad());
                }
                let r = rows.last_mut().ok_or_else(bad)?;
                if !r.description.is_empty() {
                    r.description.push(' ');
                }
                r.description.push_str(field(3, 73)?.trim());
            }
            _ => return Err(bad()),
        }
    }
    Ok(rows)
}
pub(crate) fn parse(f: &StatementInput, iban: &str, currency: &str) -> Result<Vec<MovementData>> {
    parse_inner(f, iban, currency, false)
}
pub(crate) fn parse_connection(
    f: &StatementInput,
    iban: &str,
    currency: &str,
) -> Result<Vec<MovementData>> {
    parse_inner(f, iban, currency, true)
}
fn parse_inner(
    f: &StatementInput,
    iban: &str,
    currency: &str,
    allow_empty: bool,
) -> Result<Vec<MovementData>> {
    if f.content_base64.len() > 1_398_104 || f.filename.len() > 255 {
        return Err(invalid("statement_size"));
    }
    let bytes = STANDARD.decode(&f.content_base64).map_err(|_| bad())?;
    if bytes.is_empty() || bytes.len() > 1024 * 1024 {
        return Err(invalid("statement_size"));
    }
    let decoded = match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_owned(),
        Err(_) if f.format == "rb_abo" || f.format == "moneta_xml" => {
            let (s, _, errors) = encoding_rs::WINDOWS_1250.decode(&bytes);
            if errors {
                return Err(bad());
            }
            s.into_owned()
        }
        Err(_) => return Err(bad()),
    };
    let text = decoded.trim_start_matches('\u{feff}');
    let mut rows = match f.format.as_str() {
        "camt053" => camt(text, iban, currency),
        "fio_json" => fio(text, iban, currency),
        "moneta_xml" => moneta(text, iban, currency),
        "rb_abo" => rb(text, iban, &f.filename),
        _ => Err(bad()),
    }?;
    if (!allow_empty && rows.is_empty()) || rows.len() > 1000 {
        return Err(invalid("statement_rows"));
    }
    for r in &mut rows {
        r.normalize();
        if r.currency != currency {
            return Err(invalid("currency"));
        }
        let errors = r.validate();
        if !errors.is_empty() {
            return Err(Error::Invalid(errors));
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(f: &StatementInput, iban: &str) -> Result<Vec<MovementData>> {
        super::parse(f, iban, "CZK")
    }
    const IBAN: &str = "CZ6508000000192000145399";
    const CAMT: &str = include_str!("../../../tests/fixtures/bank/camt053.xml");
    fn input(format: &str, s: &str) -> StatementInput {
        StatementInput {
            account_id: "1".into(),
            format: format.into(),
            filename: "26_001_13825001_CZK.gpc".into(),
            content_base64: STANDARD.encode(s),
        }
    }
    #[test]
    fn foreign_statements_validate_account_and_booking_currency() {
        for (format, fixture) in [
            ("camt053", CAMT),
            (
                "fio_json",
                include_str!("../../../tests/fixtures/bank/fio.json"),
            ),
            (
                "moneta_xml",
                include_str!("../../../tests/fixtures/bank/moneta.xml"),
            ),
        ] {
            let text = fixture.replace("CZK", "EUR");
            let f = input(format, &text);
            let rows = super::parse(&f, IBAN, "EUR").unwrap();
            assert!(!rows.is_empty());
            assert!(rows.iter().all(|r| r.currency == "EUR"));
            assert!(super::parse(&f, IBAN, "CZK").is_err());
        }
        let mixed = CAMT.replace("Ccy=\"CZK\"", "Ccy=\"EUR\"");
        assert!(super::parse(&input("camt053", &mixed), IBAN, "CZK").is_err());
    }
    #[test]
    fn camt_contract_and_rejections() {
        let rows = parse(&input("camt053", CAMT), IBAN).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].counterparty, "Test & Syn");
        assert_eq!(rows[0].variable_symbol, "000123");
        assert_eq!(rows[1].amount, "-25.00");
        assert!(parse(&input("camt053", CAMT), "CZ0000000000000000000000").is_err());
        for (from, to) in [
            ("100.01", "100.001"),
            ("CZK", "XXX"),
            ("BOOK", "PDNG"),
            ("CRDT", "OTHER"),
            ("synthetic-bank-1", "NOTPROVIDED"),
            ("</Stmt>", "</Wrong>"),
        ] {
            assert!(
                parse(&input("camt053", &CAMT.replace(from, to)), IBAN).is_err(),
                "{from}"
            );
        }
        let batch = CAMT.replace("</TxDtls>", "</TxDtls><TxDtls/>");
        assert!(parse(&input("camt053", &batch), IBAN).is_err());
        assert!(xml("<!DOCTYPE x [<!ENTITY a SYSTEM 'file:///etc/passwd'>]><x>&a;</x>").is_err());
        assert!(xml("<x>&unknown;</x>").is_err());
        assert!(xml(&format!("{}{}", "<x>".repeat(40), "</x>".repeat(40))).is_err());
        assert!(xml("<x/><x/>").is_err());
    }
    #[test]
    fn fio_preserves_decimal_and_leading_zeroes() {
        let s = include_str!("../../../tests/fixtures/bank/fio.json");
        let r = parse(&input("fio_json", s), IBAN).unwrap();
        assert_eq!(r[0].amount, "999999999999.99");
        assert_eq!(r[0].variable_symbol, "000123");
        assert_eq!(r[0].external_id, "123456789012");
        let r = parse(
            &input(
                "fio_json",
                &s.replace("\"2026-10-01+0200\"", "1340661600000"),
            ),
            IBAN,
        )
        .unwrap();
        assert_eq!(r[0].booked_on.to_string(), "2012-06-26");
        assert!(parse(
            &input("fio_json", &s.replace("999999999999.99", "100.001")),
            IBAN
        )
        .is_err());
    }
    #[test]
    fn moneta_requires_explicit_direction() {
        let s = include_str!("../../../tests/fixtures/bank/moneta.xml");
        let r = parse(&input("moneta_xml", s), IBAN).unwrap();
        assert_eq!(r[0].amount, "-50.01");
        assert!(parse(
            &input("moneta_xml", &s.replace("debit", "description")),
            IBAN
        )
        .is_err());
        assert!(parse(
            &input("moneta_xml", &s.replace("trn-cnt=\"1\"", "trn-cnt=\"2\"")),
            IBAN
        )
        .is_err());
    }
    #[test]
    fn rb_fixed_width_cents_account_and_original_currency_filename() {
        let s = include_str!("../../../tests/fixtures/bank/rb_CZK.gpc");
        let iban = "CZ0055000000000013825001";
        let r = parse(&input("rb_abo", s), iban).unwrap();
        assert_eq!(r[0].amount, "100.01");
        assert_eq!(r[0].booked_on.to_string(), "2026-10-01");
        let mut f = input("rb_abo", s);
        f.filename = "statement.gpc".into();
        assert!(parse(&f, iban).is_err());
        assert!(parse(&input("rb_abo", s), IBAN).is_err());
        assert!(parse(
            &input("rb_abo", &s.replace("0000000100012", "0000000100013")),
            iban
        )
        .is_err());
    }
}

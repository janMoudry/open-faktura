#[path = "inbox_qr.rs"]
mod qr;
pub(crate) use qr::image as extract_qr;
// Deliberately bounded unsigned domestic ISDOC subset; not an XSD/signature validator.
use crate::bank_statement::{xml, Node};
use faktura_application::company::{CompanyError as Error, Result};
use faktura_domain::{
    contact::Address,
    invoice::PartySnapshot,
    received::{amount, money, Data, Kind, Line},
};
fn bad() -> Error {
    Error::Invalid(vec!["isdoc_unsupported".into()])
}
fn required(n: &Node, path: &str) -> Result<String> {
    let v = n.value(path);
    if v.is_empty() {
        Err(bad())
    } else {
        Ok(v.into())
    }
}
/// Conservative classic PDF subset: no object/xref streams, incremental revisions,
/// encryption or escaped PDF names. Embedded payload inflation is bounded separately.
pub(crate) fn extract_pdf(bytes: &[u8], today: chrono::NaiveDate) -> Result<Data> {
    use std::io::Read;
    const LIMIT: usize = 2 * 1024 * 1024;
    if bytes.len() > LIMIT || !bytes.starts_with(b"%PDF-") {
        return Err(bad());
    }
    // These constructs can cause work during parser loading, before we can bound
    // the attachment. Reject escaped names too, so they cannot hide a construct.
    for token in [b"/Prev".as_slice(), b"/ObjStm", b"/XRef", b"/Encrypt"] {
        if bytes.windows(token.len()).any(|v| v == token) {
            return Err(bad());
        }
    }
    if bytes
        .windows(3)
        .any(|v| v[0] == b'#' && v[1].is_ascii_hexdigit() && v[2].is_ascii_hexdigit())
    {
        return Err(bad());
    }
    // Limit recursive PDF containers before parsing. Counting also inside streams
    // is intentionally conservative; unsupported documents retain manual entry.
    if bytes.windows(2).filter(|w| *w == b"<<").count()
        + bytes.iter().filter(|&&b| b == b'[').count()
        > 64
    {
        return Err(bad());
    }
    for (offset, _) in bytes
        .windows(7)
        .enumerate()
        .filter(|(_, v)| *v == b"/Length")
    {
        let rest = &bytes[offset + 7..];
        let start = rest
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .ok_or_else(bad)?;
        let n = rest[start..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if n == 0 {
            return Err(bad());
        }
        let value: usize = std::str::from_utf8(&rest[start..start + n])
            .map_err(|_| bad())?
            .parse()
            .map_err(|_| bad())?;
        if value > LIMIT {
            return Err(bad());
        }
        let next = rest[start + n..]
            .iter()
            .find(|b| !b.is_ascii_whitespace())
            .ok_or_else(bad)?;
        if next.is_ascii_digit() || *next == b'R' {
            return Err(bad());
        }
    }
    for marker in [b"%%EOF".as_slice(), b"startxref"] {
        if bytes.windows(marker.len()).filter(|v| *v == marker).count() != 1 {
            return Err(bad());
        }
    }
    let start = bytes
        .windows(9)
        .position(|v| v == b"startxref")
        .ok_or_else(bad)?
        + 9;
    let offset = bytes[start..]
        .split(|b| b.is_ascii_whitespace())
        .find(|v| !v.is_empty())
        .ok_or_else(bad)?;
    let offset: usize = std::str::from_utf8(offset)
        .map_err(|_| bad())?
        .parse()
        .map_err(|_| bad())?;
    let xref = bytes.get(offset..).ok_or_else(bad)?;
    if !xref.starts_with(b"xref") {
        return Err(bad());
    }
    let end = xref
        .windows(7)
        .position(|v| v == b"trailer")
        .ok_or_else(bad)?;
    for line in xref[4..end]
        .split(|b| *b == b'\n' || *b == b'\r')
        .filter(|l| !l.iter().all(u8::is_ascii_whitespace))
    {
        let tokens = line
            .split(u8::is_ascii_whitespace)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if tokens.len() == 2 {
            for token in tokens {
                let n: usize = std::str::from_utf8(token)
                    .map_err(|_| bad())?
                    .parse()
                    .map_err(|_| bad())?;
                if n > 4096 {
                    return Err(bad());
                }
            }
        } else if tokens.len() != 3 {
            return Err(bad());
        }
    }
    let pdf = lopdf::Document::load_mem(bytes).map_err(|_| bad())?;
    if pdf.objects.len() > 4096 {
        return Err(bad());
    }
    let mut payload = None;
    for object in pdf.objects.values() {
        let Ok(dict) = object.as_dict() else { continue };
        if dict.get(b"Type").and_then(|v| v.as_name()).ok() != Some(b"Filespec".as_slice()) {
            continue;
        }
        let name = dict
            .get(b"UF")
            .or_else(|_| dict.get(b"F"))
            .and_then(|v| v.as_str())
            .map_err(|_| bad())?;
        if !name.to_ascii_lowercase().ends_with(b".isdoc") {
            continue;
        }
        if payload.is_some() {
            return Err(bad());
        }
        let ef = dict
            .get(b"EF")
            .and_then(|v| v.as_dict())
            .map_err(|_| bad())?;
        let reference = ef
            .get(b"UF")
            .or_else(|_| ef.get(b"F"))
            .and_then(|v| v.as_reference())
            .map_err(|_| bad())?;
        let stream = pdf
            .objects
            .get(&reference)
            .ok_or(lopdf::Error::ObjectNotFound)
            .and_then(|v| v.as_stream())
            .map_err(|_| bad())?;
        if stream.dict.get(b"Type").and_then(|v| v.as_name()).ok()
            != Some(b"EmbeddedFile".as_slice())
        {
            return Err(bad());
        }
        let content = match stream.dict.get(b"Filter") {
            Err(_) => stream.content.clone(),
            Ok(filter) if filter.as_name().ok() == Some(b"FlateDecode".as_slice()) => {
                if stream.dict.has(b"DecodeParms") {
                    return Err(bad());
                }
                let mut output = Vec::new();
                flate2::read::ZlibDecoder::new(stream.content.as_slice())
                    .take((LIMIT + 1) as u64)
                    .read_to_end(&mut output)
                    .map_err(|_| bad())?;
                output
            }
            _ => return Err(bad()),
        };
        if content.len() > LIMIT {
            return Err(bad());
        }
        payload = Some(content);
    }
    extract(&payload.ok_or_else(bad)?, today)
}

pub(crate) fn extract(bytes: &[u8], today: chrono::NaiveDate) -> Result<Data> {
    if bytes.len() > 2097152 {
        return Err(bad());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| bad())?;
    // No DTD, signatures, alternate namespace trees or fetching referenced content.
    if text.contains("<!DOCTYPE") || text.contains("<!ENTITY") || text.matches("xmlns").count() != 1
    {
        return Err(bad());
    }
    let n = xml(text).map_err(|_| bad())?;
    if n.name != "Invoice"
        || n.attr("xmlns") != "http://isdoc.cz/namespace/2013"
        || !matches!(n.attr("version"), "6.0.1" | "6.0.2")
        || n.value("DocumentType") != "1"
        || n.value("LocalCurrencyCode") != "CZK"
    {
        return Err(bad());
    }
    let foreign = n.value("ForeignCurrencyCode");
    let exchange = if foreign.is_empty() {
        None
    } else {
        let rate = faktura_domain::exchange::ExchangeRate {
            rate: required(&n, "CurrRate")?,
            units: required(&n, "RefCurrRate")?,
        };
        if !faktura_domain::exchange::validate(foreign, Some(&rate)) {
            return Err(bad());
        }
        Some(rate)
    };
    let suffix = if exchange.is_some() { "Curr" } else { "" };
    let numeric = |v: &str| -> Result<rust_decimal::Decimal> {
        if v.len() > 64
            || v.is_empty()
            || !v
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-')
        {
            return Err(bad());
        }
        v.parse().map_err(|_| bad())
    };
    let convert = |v: rust_decimal::Decimal| -> Result<rust_decimal::Decimal> {
        if let Some(rate) = &exchange {
            let factor = numeric(&rate.rate)?
                .checked_div(numeric(&rate.units)?)
                .ok_or_else(bad)?;
            v.checked_mul(factor).ok_or_else(bad)
        } else {
            Ok(v)
        }
    };
    for path in [
        "LegalMonetaryTotal/AlreadyClaimedTaxExclusiveAmount",
        "LegalMonetaryTotal/AlreadyClaimedTaxInclusiveAmount",
        "LegalMonetaryTotal/PaidDepositsAmount",
    ] {
        if amount(n.value(path)).ok_or_else(bad)? != rust_decimal::Decimal::ZERO {
            return Err(bad());
        }
        if exchange.is_some()
            && amount(n.value(&format!("{path}Curr"))).ok_or_else(bad)?
                != rust_decimal::Decimal::ZERO
        {
            return Err(bad());
        }
    }
    let party = n.at("AccountingSupplierParty/Party").ok_or_else(bad)?;
    let supplier = PartySnapshot {
        name: required(party, "PartyName/Name")?,
        registration_number: party.value("PartyIdentification/ID").into(),
        vat_number: party.value("PartyTaxScheme/CompanyID").into(),
        address: Address {
            street: format!(
                "{} {}",
                party.value("PostalAddress/StreetName"),
                party.value("PostalAddress/BuildingNumber")
            )
            .trim()
            .into(),
            city: party.value("PostalAddress/CityName").into(),
            postal_code: party.value("PostalAddress/PostalZone").into(),
            country: required(party, "PostalAddress/Country/IdentificationCode")?,
        },
        email: party.value("Contact/ElectronicMail").into(),
        phone: party.value("Contact/Telephone").into(),
    };
    if supplier.address.country != "CZ" {
        return Err(bad());
    }
    let mut lines = vec![];
    for line in n.at("InvoiceLines").ok_or_else(bad)?.all("InvoiceLine") {
        if line
            .at("ClassifiedTaxCategory/LocalReverseCharge")
            .is_some()
            || line.value("ClassifiedTaxCategory/LocalReverseChargeFlag") == "true"
            || line.value("ClassifiedTaxCategory/LocalReverseChargeFlag") == "1"
        {
            return Err(bad());
        }
        let net = amount(line.value(&format!("LineExtensionAmount{suffix}"))).ok_or_else(bad)?;
        let gross = amount(line.value(&format!("LineExtensionAmountTaxInclusive{suffix}")))
            .ok_or_else(bad)?;
        let vat = if exchange.is_some() {
            gross - net
        } else {
            amount(line.value("LineExtensionTaxAmount")).ok_or_else(bad)?
        };
        if net + vat != gross
            || numeric(line.value("LineExtensionAmount"))? != convert(net)?
            || numeric(line.value("LineExtensionAmountTaxInclusive"))? != convert(gross)?
            || numeric(line.value("LineExtensionTaxAmount"))? != convert(vat)?
        {
            return Err(bad());
        }
        lines.push(Line {
            stock: None,
            description: required(line, "Item/Description")?,
            rate: required(line, "ClassifiedTaxCategory/Percent")?,
            net: money(net),
            vat: money(vat),
        });
        if lines.len() > 100 {
            return Err(bad());
        }
    }
    let issued_on = required(&n, "IssueDate")?.parse().map_err(|_| bad())?;
    let due = n.value("PaymentMeans/Payment/Details/PaymentDueDate");
    let payment = n.at("PaymentMeans").and_then(|v| v.all("Payment").next());
    if n.at("PaymentMeans")
        .is_some_and(|v| v.all("Payment").count() > 1)
    {
        return Err(bad());
    }
    if payment.is_some_and(|p| {
        amount(p.value("PaidAmount")).is_none_or(|v| v != rust_decimal::Decimal::ZERO)
    }) {
        return Err(bad());
    }
    let data = Data {
        exchange_rate: exchange.clone(),
        stock_incidental_cost: None,
        kind: Kind::Invoice,
        receipt_method: None,
        supplier_contact_id: None,
        supplier,
        external_number: required(&n, "ID")?,
        variable_symbol: n
            .value("PaymentMeans/Payment/Details/VariableSymbol")
            .into(),
        bank_account: {
            let p = "PaymentMeans/Payment/Details";
            let iban = n.value(&format!("{p}/IBAN"));
            if !iban.is_empty() {
                iban.into()
            } else {
                let account = n.value(&format!("{p}/ID"));
                let bank = n.value(&format!("{p}/BankCode"));
                if account.is_empty() {
                    String::new()
                } else {
                    format!("{account}/{bank}")
                }
            }
        },
        issued_on,
        received_on: today,
        due_on: if due.is_empty() {
            issued_on
        } else {
            due.parse().map_err(|_| bad())?
        },
        tax_on: if n.value("VATApplicable") == "true" || n.value("VATApplicable") == "1" {
            Some(required(&n, "TaxPointDate")?.parse().map_err(|_| bad())?)
        } else {
            None
        },
        currency: if foreign.is_empty() { "CZK" } else { foreign }.into(),
        description: String::new(),
        note: n.value("Note").into(),
        lines,
        rounding: if n
            .value(&format!("LegalMonetaryTotal/PayableRoundingAmount{suffix}"))
            .is_empty()
        {
            "0.00".into()
        } else {
            required(
                &n,
                &format!("LegalMonetaryTotal/PayableRoundingAmount{suffix}"),
            )?
        },
    };
    let totals = faktura_domain::received::totals(&data).map_err(|_| bad())?;
    if amount(&totals.net).ok_or_else(bad)? + amount(&totals.vat).ok_or_else(bad)?
        != amount(n.value(&format!("LegalMonetaryTotal/TaxInclusiveAmount{suffix}")))
            .ok_or_else(bad)?
    {
        return Err(bad());
    }
    if amount(&totals.net)
        != amount(n.value(&format!("LegalMonetaryTotal/TaxExclusiveAmount{suffix}")))
        || amount(&totals.total)
            != amount(n.value(&format!("LegalMonetaryTotal/PayableAmount{suffix}")))
        || amount(&totals.vat) != amount(n.value(&format!("TaxTotal/TaxAmount{suffix}")))
    {
        return Err(bad());
    }
    if exchange.is_some() {
        for (path, value) in [
            ("LegalMonetaryTotal/TaxExclusiveAmount", &totals.net),
            (
                "LegalMonetaryTotal/TaxInclusiveAmount",
                &money(amount(&totals.net).ok_or_else(bad)? + amount(&totals.vat).ok_or_else(bad)?),
            ),
            ("LegalMonetaryTotal/PayableAmount", &totals.total),
            ("TaxTotal/TaxAmount", &totals.vat),
        ] {
            if numeric(n.value(path))? != convert(amount(value).ok_or_else(bad)?)? {
                return Err(bad());
            }
        }
    }
    Ok(data)
}
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../tests/fixtures/inbox/synthetic-basic.isdoc");
    fn pdf_attachment(content: &[u8], compressed: bool) -> Vec<u8> {
        use lopdf::{dictionary, Object, Stream};
        let mut pdf = lopdf::Document::with_version("1.4");
        let mut stream = Stream::new(dictionary! {"Type" => "EmbeddedFile"}, content.to_vec());
        if compressed {
            stream.compress().unwrap();
        }
        let embedded = pdf.add_object(stream);
        pdf.add_object(dictionary! {"Type" => "Filespec", "F" => Object::string_literal("invoice.isdoc"), "EF" => dictionary! {"F" => embedded}});
        let root = pdf.add_object(dictionary! {"Type" => "Catalog"});
        pdf.trailer.set("Root", root);
        let mut out = Vec::new();
        pdf.save_to(&mut out).unwrap();
        out
    }
    #[test]
    fn embedded_plain_and_compressed_isdoc_preserve_source() {
        for compressed in [false, true] {
            let pdf = pdf_attachment(FIXTURE.as_bytes(), compressed);
            let data = extract_pdf(&pdf, "2026-10-18".parse().unwrap()).unwrap();
            assert_eq!(data.external_number, "SYNTHETIC-001");
            assert_eq!(data.lines[0].vat, "21.02");
        }
    }
    #[test]
    fn pdf_rejects_unsafe_container_and_inflation() {
        let today = "2026-10-18".parse().unwrap();
        for token in ["/Prev", "/XRef", "/ObjStm", "/Encrypt", "/Ty#70e"] {
            let mut pdf = pdf_attachment(FIXTURE.as_bytes(), false);
            pdf.extend_from_slice(token.as_bytes());
            assert!(extract_pdf(&pdf, today).is_err());
        }
        let bomb = pdf_attachment(&vec![b'x'; 2 * 1024 * 1024 + 1], true);
        assert!(extract_pdf(&bomb, today).is_err());
        assert!(extract_pdf(b"%PDF-1.4 malformed", today).is_err());
    }
    #[test]
    fn roundtrip_current_export_domestic_and_foreign_preserves_source_amounts() {
        use faktura_domain::invoice;
        for currency in ["CZK", "JPY"] {
            let mut data: invoice::DraftData =
                serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                    .unwrap();
            data.calculation.currency = currency.into();
            data.calculation.exchange_rate = if currency == "CZK" {
                None
            } else {
                Some(faktura_domain::exchange::ExchangeRate {
                    rate: "15.25".into(),
                    units: "100".into(),
                })
            };
            let totals = invoice::calculate(&data.calculation).unwrap();
            let expected = totals.payable.clone();
            let draft = faktura_application::draft::Draft {
                id: "1".into(),
                company_id: "1".into(),
                revision: 2,
                data,
                totals: Some(totals),
                number: Some("SYNTHETIC-EXPORT".into()),
                issued_at: Some("2026-10-18T00:00:00Z".into()),
                settlement: None,
                advances: None,
            };
            let exported =
                faktura_application::isdoc::render(&draft, "00000000-0000-4000-8000-000000000001")
                    .unwrap();
            let imported = extract(&exported.bytes, "2026-10-18".parse().unwrap()).unwrap();
            assert_eq!(imported.currency, currency);
            assert_eq!(
                faktura_domain::received::totals(&imported).unwrap().total,
                expected
            );
            assert_eq!(imported.exchange_rate.is_some(), currency != "CZK");
        }
    }
    #[test]
    fn qr_invoice_and_combined_export_import_roundtrip() {
        use faktura_domain::invoice::{DraftData, QrFormat};
        let mut data: DraftData =
            serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap();
        data.payment.method = "bank_transfer".into();
        data.payment.iban = "CZ6508000000192000145399".into();
        let totals = faktura_domain::invoice::calculate(&data.calculation).unwrap();
        for format in [QrFormat::Invoice, QrFormat::PaymentInvoice] {
            data.qr_format = format;
            let encoded = faktura_domain::invoice_qr::payload(&data, &totals, "SYNTH*001").unwrap();
            let decoded = qr::parse(&encoded, "2026-10-18".parse().unwrap()).unwrap();
            assert_eq!(decoded.external_number, "SYNTH*001");
            assert_eq!(decoded.lines[0].net, totals.amounts.net);
            assert_eq!(decoded.lines[0].vat, totals.amounts.vat);
        }
    }
    #[test]
    fn exact_source_amounts_no_recalculation() {
        let d = extract(FIXTURE.as_bytes(), "2026-10-15".parse().unwrap()).unwrap();
        assert_eq!(d.supplier.name, "Synthetic & Company");
        assert_eq!(d.variable_symbol, "000123");
        assert_eq!(d.lines[0].vat, "21.02");
        assert_eq!(d.rounding, "-0.03");
    }
    #[test]
    fn reject_entities_foreign_currency_reverse_charge_and_wrong_total() {
        for input in [
            FIXTURE.replace("121.00", "121.01"),
            FIXTURE.replace("<LocalCurrencyCode>CZK", "<LocalCurrencyCode>EUR"),
            FIXTURE.replace(
                "<Percent>21</Percent>",
                "<Percent>21</Percent><LocalReverseCharge/>",
            ),
            FIXTURE.replace(
                "<Invoice xmlns",
                "<!DOCTYPE Invoice SYSTEM \"https://example.test/external\"><Invoice xmlns",
            ),
            FIXTURE.replace("<PaidDepositsAmount>0", "<PaidDepositsAmount>1"),
            FIXTURE.replace("<DocumentType>1", "<DocumentType>2"),
        ] {
            assert!(extract(input.as_bytes(), "2026-10-15".parse().unwrap()).is_err());
        }
    }
}

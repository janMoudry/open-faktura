//! Deliberately bounded unsigned domestic ISDOC subset; not an XSD/signature validator.
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
        || !n.value("ForeignCurrencyCode").is_empty()
    {
        return Err(bad());
    }
    for path in [
        "LegalMonetaryTotal/AlreadyClaimedTaxExclusiveAmount",
        "LegalMonetaryTotal/AlreadyClaimedTaxInclusiveAmount",
        "LegalMonetaryTotal/PaidDepositsAmount",
    ] {
        if amount(n.value(path)).ok_or_else(bad)? != rust_decimal::Decimal::ZERO {
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
        let net = amount(line.value("LineExtensionAmount")).ok_or_else(bad)?;
        let vat = amount(line.value("LineExtensionTaxAmount")).ok_or_else(bad)?;
        if net + vat != amount(line.value("LineExtensionAmountTaxInclusive")).ok_or_else(bad)? {
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
        currency: "CZK".into(),
        description: String::new(),
        note: n.value("Note").into(),
        lines,
        rounding: if n
            .value("LegalMonetaryTotal/PayableRoundingAmount")
            .is_empty()
        {
            "0.00".into()
        } else {
            required(&n, "LegalMonetaryTotal/PayableRoundingAmount")?
        },
    };
    let totals = faktura_domain::received::totals(&data).map_err(|_| bad())?;
    if amount(&totals.net).ok_or_else(bad)? + amount(&totals.vat).ok_or_else(bad)?
        != amount(n.value("LegalMonetaryTotal/TaxInclusiveAmount")).ok_or_else(bad)?
    {
        return Err(bad());
    }
    if amount(&totals.net) != amount(n.value("LegalMonetaryTotal/TaxExclusiveAmount"))
        || amount(&totals.total) != amount(n.value("LegalMonetaryTotal/PayableAmount"))
        || amount(&totals.vat) != amount(n.value("TaxTotal/TaxAmount"))
    {
        return Err(bad());
    }
    Ok(data)
}
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../tests/fixtures/inbox/synthetic-basic.isdoc");
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

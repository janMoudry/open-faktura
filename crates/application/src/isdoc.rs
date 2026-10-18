//! ISDOC 6.0.2 export of immutable issued invoice snapshots.
use crate::{
    company::{CompanyError as Error, Result},
    draft::Draft,
    export::File,
};
use async_trait::async_trait;
use faktura_domain::invoice::{PartySnapshot, PriceMode, TaxMode};
use rust_decimal::Decimal;
use std::str::FromStr;
#[async_trait]
pub trait IsdocStore: Send + Sync {
    async fn export(&self, actor: &str, company: &str, id: &str) -> Result<File>;
}
fn invalid() -> Error {
    Error::Invalid(vec!["isdoc_unsupported".into()])
}
fn escape(v: &str) -> String {
    v.chars()
        .filter(|c| {
            *c == '\t'
                || *c == '\n'
                || *c == '\r'
                || (*c >= ' ' && *c != '\u{fffe}' && *c != '\u{ffff}')
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn tag(out: &mut String, name: &str, value: &str) {
    out.push_str(&format!("<{name}>{}</{name}>", escape(value)));
}
fn party(out: &mut String, kind: &str, p: &PartySnapshot) {
    out.push_str(&format!("<{kind}><Party><PartyIdentification>"));
    tag(out, "ID", &p.registration_number);
    out.push_str("</PartyIdentification><PartyName>");
    tag(out, "Name", &p.name);
    out.push_str("</PartyName><PostalAddress>");
    tag(out, "StreetName", &p.address.street);
    tag(out, "BuildingNumber", "");
    tag(out, "CityName", &p.address.city);
    tag(out, "PostalZone", &p.address.postal_code);
    out.push_str("<Country>");
    tag(out, "IdentificationCode", &p.address.country);
    tag(out, "Name", &p.address.country);
    out.push_str("</Country></PostalAddress>");
    if !p.vat_number.is_empty() {
        out.push_str("<PartyTaxScheme>");
        tag(out, "CompanyID", &p.vat_number);
        tag(out, "TaxScheme", "VAT");
        out.push_str("</PartyTaxScheme>");
    }
    out.push_str(&format!("</Party></{kind}>"));
}
pub fn render(d: &Draft, document_uuid: &str) -> Result<File> {
    let number = d.number.as_deref().ok_or_else(invalid)?;
    let t = d.totals.as_ref().ok_or_else(invalid)?;
    let data = &d.data;
    // Advance deduction has its own tax breakdown and references; never silently discard it.
    if d.advances.is_some() || uuid::Uuid::parse_str(document_uuid).is_err() {
        return Err(invalid());
    }
    let foreign = t.currency != "CZK";
    if !faktura_domain::exchange::validate(&t.currency, data.calculation.exchange_rate.as_ref()) {
        return Err(invalid());
    }
    let factor = if let Some(rate) = &data.calculation.exchange_rate {
        Decimal::from_str(&rate.rate).map_err(|_| invalid())?
            / Decimal::from_str(&rate.units).map_err(|_| invalid())?
    } else {
        Decimal::ONE
    };
    let local = |v: &str| -> Result<String> {
        Ok(Decimal::from_str(v)
            .map_err(|_| invalid())?
            .checked_mul(factor)
            .ok_or_else(invalid)?
            .normalize()
            .to_string())
    };
    let customer = data.customer.as_ref().ok_or_else(invalid)?;
    let issue = data.issued_on.ok_or_else(invalid)?;
    if data.calculation.lines.is_empty()
        || data.calculation.lines.len() != t.lines.len()
        || t.tax_summary.is_empty()
    {
        return Err(invalid());
    }
    let mut out=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><Invoice xmlns=\"http://isdoc.cz/namespace/2013\" version=\"6.0.2\">");
    tag(&mut out, "DocumentType", "1");
    tag(&mut out, "ID", number);
    tag(&mut out, "UUID", document_uuid);
    tag(&mut out, "IssuingSystem", "Open Faktura");
    tag(&mut out, "IssueDate", &issue.to_string());
    if let Some(tax) = data.tax_on {
        tag(&mut out, "TaxPointDate", &tax.to_string());
    }
    let vat = if data.calculation.tax_mode == TaxMode::VatPayer {
        "true"
    } else {
        "false"
    };
    tag(&mut out, "VATApplicable", vat);
    tag(&mut out, "ElectronicPossibilityAgreementReference", "");
    if !data.note.is_empty() {
        tag(&mut out, "Note", &data.note);
    }
    tag(&mut out, "LocalCurrencyCode", "CZK");
    if foreign {
        tag(&mut out, "ForeignCurrencyCode", &t.currency);
    }
    tag(
        &mut out,
        "CurrRate",
        data.calculation
            .exchange_rate
            .as_ref()
            .map_or("1", |r| r.rate.as_str()),
    );
    tag(
        &mut out,
        "RefCurrRate",
        data.calculation
            .exchange_rate
            .as_ref()
            .map_or("1", |r| r.units.as_str()),
    );
    party(&mut out, "AccountingSupplierParty", &data.supplier);
    party(&mut out, "AccountingCustomerParty", customer);
    out.push_str("<InvoiceLines>");
    for (idx, line) in data.calculation.lines.iter().enumerate() {
        let stored = t
            .lines
            .iter()
            .find(|l| l.id == line.id)
            .ok_or_else(invalid)?;
        let q = Decimal::from_str(&line.quantity).map_err(|_| invalid())?;
        let net = Decimal::from_str(&stored.amounts.net).map_err(|_| invalid())?;
        let gross = Decimal::from_str(&stored.amounts.gross).map_err(|_| invalid())?;
        if q <= Decimal::ZERO {
            return Err(invalid());
        }
        out.push_str("<InvoiceLine>");
        tag(&mut out, "ID", &(idx + 1).to_string());
        out.push_str(&format!(
            "<InvoicedQuantity unitCode=\"{}\">{}</InvoicedQuantity>",
            escape(&line.unit),
            escape(&line.quantity)
        ));
        if foreign {
            tag(&mut out, "LineExtensionAmountCurr", &stored.amounts.net);
        }
        tag(
            &mut out,
            "LineExtensionAmount",
            &local(&stored.amounts.net)?,
        );
        if foreign {
            tag(
                &mut out,
                "LineExtensionAmountTaxInclusiveCurr",
                &stored.amounts.gross,
            );
        }
        tag(
            &mut out,
            "LineExtensionAmountTaxInclusive",
            &local(&stored.amounts.gross)?,
        );
        tag(
            &mut out,
            "LineExtensionTaxAmount",
            &local(&stored.amounts.vat)?,
        );
        // Effective unit prices after both stored discounts; totals remain authoritative.
        tag(
            &mut out,
            "UnitPrice",
            &(net * factor / q).round_dp(8).normalize().to_string(),
        );
        tag(
            &mut out,
            "UnitPriceTaxInclusive",
            &(gross * factor / q).round_dp(8).normalize().to_string(),
        );
        out.push_str("<ClassifiedTaxCategory>");
        tag(&mut out, "Percent", &line.vat_percent);
        tag(
            &mut out,
            "VATCalculationMethod",
            if data.calculation.price_mode == PriceMode::Net {
                "0"
            } else {
                "1"
            },
        );
        tag(&mut out, "VATApplicable", vat);
        out.push_str("</ClassifiedTaxCategory><Item>");
        tag(&mut out, "Description", &line.description);
        out.push_str("</Item></InvoiceLine>");
    }
    out.push_str("</InvoiceLines><TaxTotal>");
    for summary in &t.tax_summary {
        let a = &summary.amounts;
        out.push_str("<TaxSubTotal>");
        for (name, value) in [
            ("TaxableAmount", a.net.as_str()),
            ("TaxAmount", &a.vat),
            ("TaxInclusiveAmount", &a.gross),
            ("AlreadyClaimedTaxableAmount", "0"),
            ("AlreadyClaimedTaxAmount", "0"),
            ("AlreadyClaimedTaxInclusiveAmount", "0"),
            ("DifferenceTaxableAmount", &a.net),
            ("DifferenceTaxAmount", &a.vat),
            ("DifferenceTaxInclusiveAmount", &a.gross),
        ] {
            if foreign {
                tag(&mut out, &format!("{name}Curr"), value);
            }
            tag(&mut out, name, &local(value)?);
        }
        out.push_str("<TaxCategory>");
        tag(&mut out, "Percent", &summary.vat_percent);
        tag(&mut out, "VATApplicable", vat);
        out.push_str("</TaxCategory></TaxSubTotal>");
    }
    if foreign {
        tag(&mut out, "TaxAmountCurr", &t.amounts.vat);
    }
    tag(&mut out, "TaxAmount", &local(&t.amounts.vat)?);
    out.push_str("</TaxTotal><LegalMonetaryTotal>");
    for (name, value) in [
        ("TaxExclusiveAmount", t.amounts.net.as_str()),
        ("TaxInclusiveAmount", &t.amounts.gross),
        ("AlreadyClaimedTaxExclusiveAmount", "0"),
        ("AlreadyClaimedTaxInclusiveAmount", "0"),
        ("DifferenceTaxExclusiveAmount", &t.amounts.net),
        ("DifferenceTaxInclusiveAmount", &t.amounts.gross),
        ("PayableRoundingAmount", &t.rounding),
        ("PaidDepositsAmount", "0"),
        ("PayableAmount", &t.payable),
    ] {
        tag(&mut out, name, &local(value)?);
        if foreign {
            tag(&mut out, &format!("{name}Curr"), value);
        }
    }
    out.push_str("</LegalMonetaryTotal>");
    let payment = &data.payment;
    let code = match payment.method.as_str() {
        "bank_transfer" => "42",
        "cash" => "10",
        "card" => "48",
        _ => return Err(invalid()),
    };
    out.push_str("<PaymentMeans><Payment>");
    tag(&mut out, "PaidAmount", "0");
    tag(&mut out, "PaymentMeansCode", code);
    if payment.method == "bank_transfer" {
        out.push_str("<Details>");
        tag(
            &mut out,
            "PaymentDueDate",
            &data.due_on.ok_or_else(invalid)?.to_string(),
        );
        let (account, bank) = payment
            .bank_account
            .split_once('/')
            .unwrap_or((&payment.bank_account, ""));
        for (n, v) in [
            ("ID", account),
            ("BankCode", bank),
            ("Name", ""),
            ("IBAN", &payment.iban),
            ("BIC", ""),
        ] {
            tag(&mut out, n, v);
        }
        if !payment.variable_symbol.is_empty() {
            tag(&mut out, "VariableSymbol", &payment.variable_symbol);
        }
        out.push_str("</Details>");
    }
    out.push_str("</Payment></PaymentMeans></Invoice>");
    Ok(File {
        filename: format!("invoice-{}.isdoc", d.id),
        media_type: "application/xml",
        bytes: out.into_bytes(),
    })
}

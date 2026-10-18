use image::GenericImageView;
// Version 1 PDF: only persisted snapshots and decimal strings, no live lookups.
use faktura_application::draft::Draft;
use faktura_domain::invoice::{PriceMode, TaxMode};
use genpdf::{
    elements::{Break, Paragraph},
    fonts::{FontData, FontFamily},
    style::Style,
    Element,
};

// Break pathological unspaced descriptions without truncating their contents.
fn text(value: &str) -> String {
    let mut result = String::new();
    let mut run = 0;
    for c in value.chars() {
        if c.is_whitespace() {
            run = 0;
        } else {
            run += 1;
        }
        if run > 40 {
            result.push(' ');
            run = 1;
        }
        if !c.is_control() || c == '\n' {
            result.push(c);
        }
    }
    result
}
// Reserve enough space to keep a normal quotation line's heading and amounts together.
// Long descriptions may still flow across pages; a new page always makes progress.
struct QuoteLineSpace(f64);
impl Element for QuoteLineSpace {
    fn render(
        &mut self,
        _: &genpdf::Context,
        area: genpdf::render::Area<'_>,
        _: Style,
    ) -> Result<genpdf::RenderResult, genpdf::error::Error> {
        Ok(if area.size().height < genpdf::Mm::from(self.0) {
            genpdf::RenderResult {
                size: genpdf::Size::new(1, 0),
                has_more: true,
            }
        } else {
            genpdf::RenderResult::default()
        })
    }
}
pub fn render(invoice: &Draft) -> Result<Vec<u8>, genpdf::error::Error> {
    render_with_style(invoice, None)
}
pub fn render_with_style(
    invoice: &Draft,
    style: Option<&faktura_application::document_style::Style>,
) -> Result<Vec<u8>, genpdf::error::Error> {
    render_document(invoice, false, false, None, None, style)
}
pub fn render_quote(invoice: &Draft) -> Result<Vec<u8>, genpdf::error::Error> {
    render_document(invoice, true, false, None, None, None)
}
pub fn render_quote_with_style(
    invoice: &Draft,
    style: Option<&faktura_application::document_style::Style>,
) -> Result<Vec<u8>, genpdf::error::Error> {
    if style.is_none() {
        render_quote(invoice)
    } else {
        render_document(invoice, true, false, None, None, style)
    }
}
pub fn render_advance(invoice: &Draft) -> Result<Vec<u8>, genpdf::error::Error> {
    render_advance_with_style(invoice, None)
}
pub fn render_advance_with_style(
    invoice: &Draft,
    style: Option<&faktura_application::document_style::Style>,
) -> Result<Vec<u8>, genpdf::error::Error> {
    render_document(invoice, false, true, None, None, style)
}
fn render_document(
    invoice: &Draft,
    quote: bool,
    advance: bool,
    tax_receipt: Option<&str>,
    credit: Option<&str>,
    style: Option<&faktura_application::document_style::Style>,
) -> Result<Vec<u8>, genpdf::error::Error> {
    let regular = FontData::new(
        include_bytes!("../assets/NotoSans-Regular.ttf").to_vec(),
        None,
    )?;
    let bold = FontData::new(include_bytes!("../assets/NotoSans-Bold.ttf").to_vec(), None)?;
    let mut doc = genpdf::Document::new(FontFamily {
        regular: regular.clone(),
        bold: bold.clone(),
        italic: regular,
        bold_italic: bold,
    });
    let d = &invoice.data;
    let en = d.language == "en";
    let label = |cs: &str, en_text: &str| {
        if en {
            en_text.to_owned()
        } else {
            cs.to_owned()
        }
    };
    let number = invoice.number.as_deref().unwrap_or("");
    let totals = invoice.totals.as_ref().expect("issued invoice totals");
    let heading = format!(
        "{} {number}",
        if credit.is_some() {
            if d.calculation.tax_mode == TaxMode::VatPayer {
                label("Opravný daňový doklad", "Credit note")
            } else {
                label("Dobropis", "Credit note")
            }
        } else if tax_receipt.is_some() {
            label(
                "Daňový doklad k přijaté platbě",
                "Tax receipt for advance payment",
            )
        } else if advance {
            label("Zálohová faktura", "Proforma invoice")
        } else if quote {
            label("Cenová nabídka", "Quotation")
        } else {
            label("Faktura", "Invoice")
        }
    );
    doc.set_title(&heading);
    doc.set_font_size(10);
    doc.set_line_spacing(1.3);
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(18);
    let header = heading.clone();
    decorator.set_header(move |page| {
        Paragraph::new(format!("{header}  ·  {page}")).styled(Style::new().with_font_size(8))
    });
    doc.set_page_decorator(decorator);
    if let Some(logo) = style.and_then(|s| s.logo_base64.as_deref()) {
        doc.push(style_image(logo, 40.0)?);
    }
    doc.push(Break::new(1));
    doc.push(Paragraph::new(heading).styled(Style::new().bold().with_font_size(22)));
    if let Some(number) = credit {
        doc.push(Paragraph::new(format!(
            "{}: {number}",
            label("Oprava faktury", "Correction of invoice")
        )));
    }
    if let Some(number) = tax_receipt {
        doc.push(Paragraph::new(format!(
            "{}: {number}",
            label("Zálohová faktura", "Proforma invoice")
        )));
        doc.push(Paragraph::new(label(
            "Již uhrazeno — neplaťte znovu",
            "Already paid — do not pay again",
        )));
    }
    if advance {
        doc.push(Paragraph::new(label(
            "Není daňovým dokladem",
            "Not a tax invoice",
        )));
    }
    if credit.is_none() && !quote && !advance && d.calculation.tax_mode == TaxMode::VatPayer {
        doc.push(Paragraph::new(label("Daňový doklad", "Tax invoice")));
    }
    doc.push(Break::new(1));
    for (title, party) in [
        (label("Dodavatel", "Supplier"), &d.supplier),
        (
            label("Odběratel", "Customer"),
            d.customer.as_ref().expect("issued customer"),
        ),
    ] {
        doc.push(Paragraph::new(title).styled(Style::new().bold()));
        doc.push(Paragraph::new(text(&party.name)));
        let locality = format!("{} {}", party.address.postal_code, party.address.city);
        let address = [
            party.address.street.as_str(),
            locality.as_str(),
            party.address.country.as_str(),
        ]
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
        if !address.is_empty() {
            doc.push(Paragraph::new(text(&address)));
        }
        for (key, value) in [
            (label("IČO", "Registration no."), &party.registration_number),
            (label("DIČ", "VAT ID"), &party.vat_number),
            ("E-mail".into(), &party.email),
            (label("Telefon", "Phone"), &party.phone),
        ] {
            if !value.is_empty() {
                doc.push(Paragraph::new(text(&format!("{key}: {value}"))));
            }
        }
        doc.push(Break::new(0.6));
    }
    for (key, value) in [
        (label("Datum vystavení", "Issue date"), d.issued_on),
        (
            if quote {
                label("Platnost do", "Valid until")
            } else {
                label("Datum splatnosti", "Due date")
            },
            d.due_on,
        ),
        (
            label("Datum zdanitelného plnění", "Tax date"),
            if quote || advance { None } else { d.tax_on },
        ),
    ] {
        if let Some(date) = value {
            doc.push(Paragraph::new(format!(
                "{key}: {}",
                date.format(if en { "%Y-%m-%d" } else { "%d. %m. %Y" })
            )));
        }
    }
    doc.push(Paragraph::new(format!(
        "{}: {}",
        label("Způsob úhrady", "Payment method"),
        if d.payment.method == "cash" {
            label("Hotově", "Cash")
        } else {
            label("Bankovní převod", "Bank transfer")
        }
    )));
    for (key, value) in [
        (label("Účet", "Account"), &d.payment.bank_account),
        ("IBAN".into(), &d.payment.iban),
        (
            label("Variabilní symbol", "Payment reference"),
            &d.payment.variable_symbol,
        ),
        (label("Objednávka", "Order"), &d.order_number),
    ] {
        if !value.is_empty() {
            doc.push(Paragraph::new(text(&format!("{key}: {value}"))));
        }
    }
    doc.push(Break::new(1));
    doc.push(
        Paragraph::new(if quote {
            label("Položky nabídky", "Quotation items")
        } else {
            label("Položky faktury", "Invoice items")
        })
        .styled(Style::new().bold().with_font_size(14)),
    );
    doc.push(
        Paragraph::new(if d.calculation.price_mode == PriceMode::Net {
            label("Jednotkové ceny bez DPH", "Unit prices excluding VAT")
        } else {
            label("Jednotkové ceny s DPH", "Unit prices including VAT")
        })
        .styled(Style::new().with_font_size(8)),
    );
    if let Some(rate) = &d.calculation.exchange_rate {
        doc.push(Paragraph::new(format!(
            "{}: {} CZK / {} {}",
            label("Kurz", "Exchange rate"),
            rate.rate,
            rate.units,
            totals.currency
        )));
    }
    let money = |s: &str| {
        format!(
            "{} {}",
            if en {
                s.to_owned()
            } else {
                s.replace('.', ",")
            },
            if totals.currency == "CZK" && !en {
                "Kč"
            } else {
                &totals.currency
            }
        )
    };
    if let Some(style) = style {
        let before = if en {
            &style.before_items_en
        } else {
            &style.before_items_cs
        };
        if !before.is_empty() {
            for line in before.lines() {
                doc.push(Paragraph::new(text(line)));
            }
            doc.push(Break::new(1));
        }
    }
    for (i, (line, result)) in d.calculation.lines.iter().zip(&totals.lines).enumerate() {
        if quote {
            doc.push(QuoteLineSpace(32.0));
        }
        doc.push(Break::new(0.5));
        doc.push(
            Paragraph::new(text(&format!("{}. {}", i + 1, line.description)))
                .styled(Style::new().bold()),
        );
        doc.push(Paragraph::new(text(&format!(
            "{} {} × {} · {} {} % · {} {} %",
            line.quantity,
            line.unit,
            money(&line.unit_price),
            label("Sleva", "Discount"),
            line.discount_percent,
            label("DPH", "VAT"),
            line.vat_percent
        ))));
        doc.push(Paragraph::new(format!(
            "{}: {} · {}: {} · {}: {}",
            label("Základ", "Net"),
            money(&result.amounts.net),
            label("DPH", "VAT"),
            money(&result.amounts.vat),
            label("Celkem", "Total"),
            money(&result.amounts.gross)
        )));
    }
    if let Some(settlement) = &invoice.advances {
        doc.push(Break::new(1));
        doc.push(
            Paragraph::new(label("Odpočty záloh", "Advance deductions"))
                .styled(Style::new().bold()),
        );
        for source in &settlement.sources {
            doc.push(Paragraph::new(format!(
                "{}: -{}",
                source.number,
                money(&source.paid)
            )));
            for tax in &source.tax_receipts {
                doc.push(Paragraph::new(tax.number.clone()));
                for row in &tax.totals.tax_summary {
                    doc.push(Paragraph::new(format!(
                        "{} {} %: -{} {} · -{} {}",
                        label("DPH", "VAT"),
                        row.vat_percent,
                        money(&row.amounts.net),
                        label("základ", "net"),
                        money(&row.amounts.vat),
                        label("daň", "tax")
                    )));
                }
            }
        }
        doc.push(Paragraph::new(format!(
            "{}: -{}",
            label("Zaplacené zálohy celkem", "Total advances received"),
            money(&settlement.credited)
        )));
    }
    if quote {
        doc.push(QuoteLineSpace(60.0));
    }
    doc.push(Break::new(1));
    doc.push(Paragraph::new(format!(
        "{}: {} %",
        label("Sleva na doklad", "Document discount"),
        d.calculation.discount_percent
    )));
    if d.calculation.tax_mode == TaxMode::VatPayer {
        for row in &totals.tax_summary {
            doc.push(Paragraph::new(format!(
                "{} {} %: {} {} · {} {}",
                label("DPH", "VAT"),
                row.vat_percent,
                label("základ", "net"),
                money(&row.amounts.net),
                label("daň", "tax"),
                money(&row.amounts.vat)
            )));
        }
    } else {
        doc.push(Paragraph::new(label(
            "Dodavatel není plátcem DPH.",
            "Supplier is not registered for VAT.",
        )));
    }
    doc.push(Paragraph::new(format!(
        "{}: {}",
        label("Zaokrouhlení", "Rounding"),
        money(&totals.rounding)
    )));
    doc.push(
        Paragraph::new(format!(
            "{}: {}",
            if credit.is_some() {
                label("Dobropisovaná částka", "Credited amount")
            } else if tax_receipt.is_some() {
                label("Přijatá platba celkem", "Total received payment")
            } else if quote {
                label("Cena celkem", "Total price")
            } else {
                label("Celkem k úhradě", "Total payable")
            },
            money(&totals.payable)
        ))
        .styled(Style::new().bold().with_font_size(16)),
    );
    if !quote && credit.is_none() && tax_receipt.is_none() {
        let payload = if advance || invoice.advances.is_some() {
            if d.qr_format == faktura_domain::invoice::QrFormat::Payment {
                faktura_domain::payment_qr::payload(
                    &d.payment,
                    &totals.payable,
                    &totals.currency,
                    d.due_on,
                )
            } else {
                None
            }
        } else {
            invoice
                .number
                .as_deref()
                .and_then(|number| faktura_domain::invoice_qr::payload(d, totals, number))
        };
        if let Some(payload) = payload {
            doc.push(QuoteLineSpace(48.0));
            let title = match d.qr_format {
                faktura_domain::invoice::QrFormat::Payment => label("QR Platba", "QR payment"),
                faktura_domain::invoice::QrFormat::Invoice => label("QR Faktura", "QR invoice"),
                faktura_domain::invoice::QrFormat::PaymentInvoice => {
                    label("QR Platba+F", "QR payment + invoice")
                }
            };
            doc.push(Paragraph::new(title).styled(Style::new().bold()));
            doc.push(crate::payment_qr::pdf_image(&payload)?);
            doc.push(
                Paragraph::new(label(
                    "Původní částka dokladu; zohledněte již provedené úhrady.",
                    "Original document amount; account for payments already made.",
                ))
                .styled(Style::new().with_font_size(8)),
            );
        }
    }
    if !d.note.is_empty() {
        doc.push(Break::new(1));
        doc.push(Paragraph::new(text(&d.note)));
    }
    if let Some(style) = style {
        if let Some(signature) = style.signature_base64.as_deref() {
            doc.push(Break::new(1));
            doc.push(style_image(signature, 35.0)?);
        }
        let footer = if d.language == "en" {
            &style.footer_en
        } else {
            &style.footer_cs
        };
        if !footer.is_empty() {
            doc.push(Break::new(1));
            doc.push(Paragraph::new(text(footer)).styled(Style::new().with_font_size(8)));
        }
    }
    let mut output = Vec::new();
    doc.render(&mut output)?;
    Ok(output)
}

/// Same immutable renderer, with an explicit document type and payment caption.
pub fn render_tax_receipt(
    receipt: &faktura_application::tax_receipt::Document,
) -> Result<Vec<u8>, genpdf::error::Error> {
    render_tax_receipt_with_style(receipt, None)
}
pub fn render_tax_receipt_with_style(
    receipt: &faktura_application::tax_receipt::Document,
    style: Option<&faktura_application::document_style::Style>,
) -> Result<Vec<u8>, genpdf::error::Error> {
    let draft = Draft {
        advances: None,
        id: receipt.id.clone(),
        company_id: receipt.company_id.clone(),
        revision: 1,
        data: receipt.data.clone(),
        totals: Some(receipt.totals.clone()),
        number: Some(receipt.number.clone()),
        settlement: None,
        issued_at: None,
    };
    render_document(
        &draft,
        false,
        false,
        Some(&receipt.advance_number),
        None,
        style,
    )
}

pub fn render_credit(
    credit: &faktura_application::credit_note::Document,
) -> Result<Vec<u8>, genpdf::error::Error> {
    render_credit_with_style(credit, None)
}
pub fn render_credit_with_style(
    credit: &faktura_application::credit_note::Document,
    style: Option<&faktura_application::document_style::Style>,
) -> Result<Vec<u8>, genpdf::error::Error> {
    let mut d = credit.source.clone();
    d.number = credit.number.clone();
    d.advances = None;
    d.totals = Some(credit.totals.clone());
    d.data.issued_on = Some(credit.data.issued_on);
    d.data.tax_on = Some(credit.data.tax_on);
    d.data.due_on = Some(credit.data.due_on);
    d.data.language = credit.data.language.clone();
    d.data.note = format!(
        "{}: {}",
        if credit.data.language == "en" {
            "Reason for correction"
        } else {
            "Zdůvodnění opravy"
        },
        credit.data.reason
    );
    d.data.calculation.price_mode = faktura_domain::invoice::PriceMode::Gross;
    d.data.calculation.discount_percent = "0".into();
    d.data.calculation.lines = credit
        .totals
        .lines
        .iter()
        .map(|r| {
            let mut line = credit
                .source
                .data
                .calculation
                .lines
                .iter()
                .find(|l| l.id == r.id)
                .expect("stored credit source line")
                .clone();
            line.quantity = "1".into();
            line.unit = String::new();
            line.discount_percent = "0".into();
            line.unit_price = r.amounts.gross.clone();
            line
        })
        .collect();
    render_document(&d, false, false, None, Some(&credit.invoice_number), style)
}

pub(crate) fn style_image(
    value: &str,
    max_mm: f64,
) -> Result<genpdf::elements::Image, genpdf::error::Error> {
    let image = crate::document_style::decode_image(value).map_err(|_| {
        genpdf::error::Error::new(
            "Invalid document style image",
            genpdf::error::ErrorKind::InvalidData,
        )
    })?;
    let longest = image.width().max(image.height());
    let scale = max_mm / (f64::from(longest) * 25.4 / 300.0);
    Ok(genpdf::elements::Image::from_dynamic_image(image)?.with_scale((scale, scale)))
}

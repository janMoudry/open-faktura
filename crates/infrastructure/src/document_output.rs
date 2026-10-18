use async_trait::async_trait;
use faktura_application::document_output::*;
use sqlx::{PgPool, Row};
pub struct PgOutput(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
#[async_trait]
impl OutputStore for PgOutput {
    async fn render(&self, actor: &str, company: &str, request: Request) -> Result<File> {
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let (_, cid) = crate::draft::authorize(&mut tx, actor, company, false).await?;
        if request.ids.is_empty()
            || request.ids.len() > 50
            || !["cs", "en"].contains(&request.language.as_str())
            || (!matches!(request.kind, Kind::BulkPdf) && request.ids.len() != 1)
            || (request.delivery_note_prices.is_some()
                && !matches!(request.kind, Kind::DeliveryNote))
        {
            return Err(Error::Invalid(vec!["ids".into(), "language".into()]));
        }
        let delivery_prices = if matches!(request.kind, Kind::DeliveryNote) {
            match request.delivery_note_prices {
                Some(value) => value,
                None => {
                    let style: Option<serde_json::Value> = sqlx::query_scalar(
                        "SELECT data FROM company_document_style WHERE company_id=$1",
                    )
                    .bind(cid)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(err)?;
                    style
                        .map(serde_json::from_value::<faktura_application::document_style::Style>)
                        .transpose()
                        .map_err(err)?
                        .is_some_and(|s| s.delivery_note_prices)
                }
            }
        } else {
            false
        };
        if matches!(request.kind, Kind::BulkPdf) {
            use faktura_application::draft::DraftStore;
            let mut unique = std::collections::HashSet::new();
            for input in &request.ids {
                let parsed: i64 = input.parse().map_err(|_| Error::NotFound)?;
                if !unique.insert(parsed) {
                    return Err(Error::Invalid(vec!["ids".into()]));
                }
            }
            tx.commit().await.map_err(err)?;
            let drafts = crate::draft::PgDraft(self.0.clone());
            let mut size = 0;
            for input in &request.ids {
                size += drafts.pdf(actor, company, input).await?.len();
                if size > 50 * 1024 * 1024 {
                    return Err(Error::Invalid(vec!["output_limit".into()]));
                }
            }
            tx = self.0.begin().await.map_err(err)?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
                .execute(&mut *tx)
                .await
                .map_err(err)?;
            crate::draft::authorize(&mut tx, actor, company, false).await?;
        }
        let mut seen = std::collections::HashSet::new();
        let mut pdfs = Vec::new();
        let mut lines = Vec::new();
        let mut total = 0;
        for input in &request.ids {
            let id: i64 = input.parse().map_err(|_| Error::NotFound)?;
            if !seen.insert(id) {
                return Err(Error::Invalid(vec!["ids".into()]));
            }
            let row = sqlx::query(
                "SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 AND number IS NOT NULL",
            )
            .bind(cid)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
            let pdf: Option<Vec<u8>> = row.get("pdf");
            let value = crate::draft::map(row)?;
            if matches!(request.kind, Kind::BulkPdf) {
                let pdf = pdf.ok_or(Error::Conflict)?;
                total += pdf.len();
                if total > 50 * 1024 * 1024 {
                    return Err(Error::Invalid(vec!["output_limit".into()]));
                }
                pdfs.push(pdf);
                continue;
            }
            if delivery_prices
                && value.data.calculation.lines.iter().any(|line| {
                    value
                        .totals
                        .as_ref()
                        .is_none_or(|t| !t.lines.iter().any(|r| r.id == line.id))
                })
            {
                return Err(Error::Conflict);
            }
            lines = content(&value, &request.kind, &request.language, delivery_prices);
            if matches!(request.kind, Kind::PaymentConfirmation) {
                let payments=sqlx::query("SELECT p.amount::text amount,p.paid_on::text paid_on,p.method FROM invoice_payment p WHERE p.company_id=$1 AND p.invoice_id=$2 AND NOT EXISTS(SELECT 1 FROM invoice_payment_reversal r WHERE r.company_id=p.company_id AND r.invoice_id=p.invoice_id AND r.payment_id=p.id) ORDER BY p.paid_on,p.id").bind(cid).bind(id).fetch_all(&mut *tx).await.map_err(err)?;
                if payments.is_empty() {
                    return Err(Error::Conflict);
                }
                for p in payments {
                    let method: String = p.get("method");
                    let en = request.language == "en";
                    lines.push(format!(
                        "{} | {} {} | {}",
                        p.get::<String, _>("paid_on"),
                        p.get::<String, _>("amount"),
                        value.data.calculation.currency,
                        match method.as_str() {
                            "cash" =>
                                if en {
                                    "Cash"
                                } else {
                                    "Hotovost"
                                },
                            _ =>
                                if en {
                                    "Bank transfer"
                                } else {
                                    "Bankovní převod"
                                },
                        }
                    ));
                }
            }
        }
        tx.commit().await.map_err(err)?;
        // CPU rendering is bounded across every output request and runs off the runtime.
        static RENDERERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
        let _permit = RENDERERS.acquire().await.map_err(err)?;
        let kind = request.kind;
        let (bytes, filename, media_type) = tokio::task::spawn_blocking(move || -> Result<_> {
            if matches!(kind, Kind::BulkPdf) {
                Ok((
                    merge(pdfs)?,
                    "invoices.pdf".to_owned(),
                    "application/pdf".to_owned(),
                ))
            } else if matches!(kind, Kind::Rtf) {
                Ok((
                    rtf(&lines),
                    "invoice.rtf".to_owned(),
                    "application/rtf".to_owned(),
                ))
            } else {
                let filename = if matches!(kind, Kind::DeliveryNote) {
                    "delivery-note.pdf"
                } else {
                    "payment-confirmation.pdf"
                };
                Ok((
                    pdf(&lines)?,
                    filename.to_owned(),
                    "application/pdf".to_owned(),
                ))
            }
        })
        .await
        .map_err(err)??;
        // Recheck current membership after long CPU work before releasing company data.
        let mut tx = self.0.begin().await.map_err(err)?;
        crate::draft::authorize(&mut tx, actor, company, false).await?;
        tx.commit().await.map_err(err)?;
        Ok(File {
            bytes,
            filename,
            media_type,
        })
    }
}
fn content(
    d: &faktura_application::draft::Draft,
    k: &Kind,
    language: &str,
    delivery_prices: bool,
) -> Vec<String> {
    let en = language == "en";
    let label = |cs: &str, en_text: &str| {
        if en {
            en_text.to_owned()
        } else {
            cs.to_owned()
        }
    };
    let title = match k {
        Kind::DeliveryNote => label("Dodací list", "Delivery note"),
        Kind::PaymentConfirmation => label(
            "Potvrzení evidovaných úhrad",
            "Confirmation of recorded payments",
        ),
        _ => label("Faktura – editovatelná kopie", "Invoice – editable copy"),
    };
    let mut lines = vec![
        title,
        format!(
            "{}: {}",
            label("Faktura", "Invoice"),
            d.number.as_deref().unwrap_or_default()
        ),
        format!(
            "{}: {}",
            label("Datum vystavení faktury", "Invoice issue date"),
            d.data
                .issued_on
                .map(|date| date.to_string())
                .unwrap_or_default()
        ),
    ];
    if matches!(k, Kind::Rtf) {
        lines.push(format!(
            "{}: {} | {}: {}",
            label("Splatnost", "Due date"),
            d.data.due_on.map(|v| v.to_string()).unwrap_or_default(),
            label("DUZP", "Tax date"),
            d.data.tax_on.map(|v| v.to_string()).unwrap_or_default()
        ));
        lines.push(format!(
            "{}: {} | {}: {}",
            label("Objednávka", "Order"),
            d.data.order_number,
            label("Variabilní symbol", "Variable symbol"),
            d.data.payment.variable_symbol
        ));
        lines.push(format!(
            "{}: {} | {}: {} | IBAN: {}",
            label("Úhrada", "Payment"),
            match d.data.payment.method.as_str() {
                "cash" => label("Hotovost", "Cash"),
                "bank_transfer" => label("Bankovní převod", "Bank transfer"),
                method => method.to_owned(),
            },
            label("Účet", "Account"),
            d.data.payment.bank_account,
            d.data.payment.iban
        ));
        lines.push(format!(
            "{}: {} | {}: {} %",
            label("Režim cen", "Price mode"),
            if matches!(
                d.data.calculation.price_mode,
                faktura_domain::invoice::PriceMode::Gross
            ) {
                label("S DPH", "Including VAT")
            } else {
                label("Bez DPH", "Excluding VAT")
            },
            label("Sleva dokladu", "Document discount"),
            d.data.calculation.discount_percent
        ));
    }
    for (title, p) in [
        (label("Dodavatel", "Supplier"), Some(&d.data.supplier)),
        (label("Odběratel", "Customer"), d.data.customer.as_ref()),
    ] {
        let Some(p) = p else { continue };
        lines.push(String::new());
        lines.push(title);
        lines.push(p.name.clone());
        lines.push(format!(
            "{}, {} {}",
            p.address.street, p.address.postal_code, p.address.city
        ));
        if !p.registration_number.is_empty() {
            lines.push(format!(
                "{}: {}",
                label("IČO", "Registration number"),
                p.registration_number
            ));
        }
        if !p.vat_number.is_empty() {
            lines.push(format!("{}: {}", label("DIČ", "VAT number"), p.vat_number));
        }
        if matches!(k, Kind::Rtf) {
            lines.push(format!("{} | {} | {}", p.address.country, p.email, p.phone));
        }
    }
    if matches!(k, Kind::PaymentConfirmation) {
        lines.push(label(
            "Pouze aktivní evidované peněžní úhrady. Není daňový doklad; nezahrnuje zápočty.",
            "Active recorded monetary payments only. Not a tax document; excludes offsets.",
        ));
        return lines;
    }
    lines.push(String::new());
    lines.push(label("Položky a množství", "Items and quantity"));
    if matches!(k, Kind::DeliveryNote) {
        lines.push(if delivery_prices {
            label(
                "Ceny z původní faktury; nejde o výzvu k úhradě.",
                "Prices from the original invoice; not a payment request.",
            )
        } else {
            label("Bez cen", "Prices omitted")
        });
    }
    for line in &d.data.calculation.lines {
        lines.push(format!(
            "{} | {} {}",
            line.description, line.quantity, line.unit
        ));
        if matches!(k, Kind::DeliveryNote) && delivery_prices {
            let amounts = d
                .totals
                .as_ref()
                .and_then(|t| t.lines.iter().find(|r| r.id == line.id))
                .map(|r| &r.amounts);
            if let Some(amounts) = amounts {
                lines.push(format!(
                    "{}: {} {} | {}: {} {} | {}: {} {}",
                    label("Základ položky", "Line net"),
                    if en {
                        amounts.net.clone()
                    } else {
                        amounts.net.replace('.', ",")
                    },
                    d.data.calculation.currency,
                    label("DPH položky", "Line VAT"),
                    if en {
                        amounts.vat.clone()
                    } else {
                        amounts.vat.replace('.', ",")
                    },
                    d.data.calculation.currency,
                    label("Celkem položka", "Line gross"),
                    if en {
                        amounts.gross.clone()
                    } else {
                        amounts.gross.replace('.', ",")
                    },
                    d.data.calculation.currency
                ));
            }
        }
        if matches!(k, Kind::Rtf) {
            lines.push(format!(
                "{}: {} %",
                label("Sleva položky", "Line discount"),
                line.discount_percent
            ));
            if let Some(amounts) = d
                .totals
                .as_ref()
                .and_then(|t| t.lines.iter().find(|r| r.id == line.id))
                .map(|r| &r.amounts)
            {
                lines.push(format!(
                    "{}: {} | {}: {} | {}: {} {}",
                    label("Základ položky", "Line net"),
                    amounts.net,
                    label("DPH položky", "Line VAT"),
                    amounts.vat,
                    label("Celkem položka", "Line gross"),
                    amounts.gross,
                    d.data.calculation.currency
                ));
            }
            lines.push(format!(
                "{}: {} {} | {}: {} %",
                label("Jednotková cena", "Unit price"),
                line.unit_price,
                d.data.calculation.currency,
                label("DPH", "VAT"),
                line.vat_percent
            ));
        }
    }
    if matches!(k, Kind::DeliveryNote) {
        lines.push(String::new());
        lines.push(label("Předal:", "Delivered by:"));
        lines.push(String::new());
        lines.push(label("Převzal:", "Received by:"));
        lines.push(String::new());
        lines.push(label("Datum převzetí:", "Receipt date:"));
    }
    if matches!(k, Kind::Rtf) {
        if let Some(t) = &d.totals {
            lines.push(format!(
                "{}: {} | {}: {} | {}: {} {}",
                label("Základ", "Net"),
                t.amounts.net,
                label("DPH", "VAT"),
                t.amounts.vat,
                label("K úhradě", "Payable"),
                t.payable,
                d.data.calculation.currency
            ));
        }
        if let Some(t) = &d.totals {
            for tax in &t.tax_summary {
                lines.push(format!(
                    "{} {} % | {}: {} | {}: {} | {}: {} {}",
                    label("DPH", "VAT"),
                    tax.vat_percent,
                    label("Základ", "Net"),
                    tax.amounts.net,
                    label("DPH", "VAT"),
                    tax.amounts.vat,
                    label("Celkem", "Gross"),
                    tax.amounts.gross,
                    d.data.calculation.currency
                ));
            }
            lines.push(format!(
                "{}: {} {}",
                label("Zaokrouhlení", "Rounding"),
                t.rounding,
                d.data.calculation.currency
            ));
        }
        if let Some(advances) = &d.advances {
            for source in &advances.sources {
                lines.push(format!(
                    "{} {}: {} {}",
                    label("Odpočet zálohy", "Advance deduction"),
                    source.number,
                    source.paid,
                    advances.currency
                ));
                for tax in &source.tax_receipts {
                    lines.push(format!(
                        "{} {}: {} {}",
                        label("Daňový doklad zálohy", "Advance tax receipt"),
                        tax.number,
                        tax.totals.payable,
                        advances.currency
                    ));
                }
            }
            lines.push(format!(
                "{}: {} {}",
                label("Odpočet celkem", "Total deduction"),
                advances.credited,
                advances.currency
            ));
        }
        lines.push(d.data.note.clone());
    }
    lines
}
fn rtf(lines: &[String]) -> Vec<u8> {
    let mut value = String::from("{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0 Arial;}}\\f0\\fs22\\uc1\n");
    for line in lines {
        for c in line.encode_utf16() {
            match c {
                92 | 123 | 125 => {
                    value.push('\\');
                    value.push(char::from_u32(c as u32).unwrap());
                }
                10 | 13 => value.push_str("\\line "),
                32..=126 => value.push(char::from_u32(c as u32).unwrap()),
                _ => value.push_str(&format!("\\u{}?", c as i16)),
            }
        }
        value.push_str("\\par\n");
    }
    value.push('}');
    value.into_bytes()
}
fn pdf(lines: &[String]) -> Result<Vec<u8>> {
    use genpdf::{
        elements::Paragraph,
        fonts::{FontData, FontFamily},
    };
    let regular = FontData::new(
        include_bytes!("../assets/NotoSans-Regular.ttf").to_vec(),
        None,
    )
    .map_err(err)?;
    let bold =
        FontData::new(include_bytes!("../assets/NotoSans-Bold.ttf").to_vec(), None).map_err(err)?;
    let mut document = genpdf::Document::new(FontFamily {
        regular: regular.clone(),
        bold: bold.clone(),
        italic: regular,
        bold_italic: bold,
    });
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    document.set_page_decorator(decorator);
    document.set_font_size(10);
    document.set_line_spacing(1.3);
    for (index, line) in lines.iter().enumerate() {
        if line.is_empty() {
            document.push(genpdf::elements::Break::new(1));
            continue;
        }
        let mut run = 0;
        let mut text = String::new();
        for c in line.chars().filter(|c| !c.is_control()) {
            if c.is_whitespace() {
                run = 0
            } else {
                run += 1
            }
            if run > 50 {
                text.push(' ');
                run = 1
            }
            text.push(c);
        }
        use genpdf::Element;
        if index == 0 {
            document.push(
                Paragraph::new(text).styled(genpdf::style::Style::new().bold().with_font_size(18)),
            );
            document.push(genpdf::elements::Break::new(1));
        } else if [
            "Dodavatel",
            "Supplier",
            "Odběratel",
            "Customer",
            "Položky a množství",
            "Items and quantity",
        ]
        .contains(&line.as_str())
        {
            document.push(
                Paragraph::new(text).styled(genpdf::style::Style::new().bold().with_font_size(11)),
            );
        } else {
            document.push(Paragraph::new(text));
        }
    }
    let mut bytes = Vec::new();
    document.render(&mut bytes).map_err(err)?;
    Ok(bytes)
}
fn merge(pdfs: Vec<Vec<u8>>) -> Result<Vec<u8>> {
    use lopdf::{dictionary, Document, Object};
    let mut out = Document::with_version("1.7");
    let mut roots = Vec::new();
    let mut pages = 0i64;
    for bytes in pdfs {
        let mut doc = Document::load_mem(&bytes).map_err(err)?;
        doc.renumber_objects_with(out.max_id + 1);
        let root = doc
            .catalog()
            .map_err(err)?
            .get(b"Pages")
            .map_err(err)?
            .as_reference()
            .map_err(err)?;
        pages += doc.get_pages().len() as i64;
        out.max_id = doc.max_id;
        out.objects.extend(doc.objects);
        roots.push(root);
    }
    let pages_id = out.new_object_id();
    for root in &roots {
        out.get_object_mut(*root)
            .map_err(err)?
            .as_dict_mut()
            .map_err(err)?
            .set("Parent", pages_id);
    }
    out.objects.insert(pages_id,Object::Dictionary(dictionary!{"Type"=>"Pages","Kids"=>roots.iter().map(|id|Object::Reference(*id)).collect::<Vec<_>>(),"Count"=>pages}));
    let catalog = out.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages_id});
    out.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    out.save_to(&mut bytes).map_err(err)?;
    Ok(bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rtf_escapes_control_and_unicode() {
        let out = String::from_utf8(rtf(&["Žluťoučký {test} \\".into()])).unwrap();
        assert!(out.contains("\\u381?"));
        assert!(out.contains("\\{test\\}"));
        assert!(out.contains("\\\\"));
    }
    #[test]
    fn merge_preserves_page_count() {
        let a = pdf(&["First".into()]).unwrap();
        let b = pdf(&["Second".into()]).unwrap();
        let merged = merge(vec![a, b]).unwrap();
        assert_eq!(
            lopdf::Document::load_mem(&merged)
                .unwrap()
                .get_pages()
                .len(),
            2
        );
    }
}

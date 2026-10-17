use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    contact::ContactStore,
    draft::DraftStore,
    export::{self, File, Format, Language, Request, Selection},
};
use sqlx::PgPool;
use std::io::{Cursor, Write};
pub struct PgExport(pub PgPool);
fn unavailable(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn label(key: &str, cs: bool) -> &str {
    let pair = match key {
        "number" => ("Číslo dokladu", "Document number"),
        "description" => ("Popis", "Description"),
        "status" => ("Stav", "Status"),
        "customer" => ("Odběratel", "Customer"),
        "registration_number" => ("IČO", "Registration number"),
        "issued_on" => ("Datum vystavení", "Issue date"),
        "due_on" => ("Datum splatnosti", "Due date"),
        "currency" => ("Měna", "Currency"),
        "net" => ("Bez DPH", "Net amount"),
        "vat" => ("DPH", "VAT"),
        "payable" => ("Celkem k úhradě", "Total payable"),
        "paid" => ("Uhrazeno", "Paid"),
        "outstanding" => ("Zbývá uhradit", "Outstanding"),
        "variable_symbol" => ("Variabilní symbol", "Variable symbol"),
        "name" => ("Název", "Name"),
        "vat_number" => ("DIČ", "VAT number"),
        "email" => ("E-mail", "Email"),
        "phone" => ("Telefon", "Phone"),
        "street" => ("Ulice", "Street"),
        "city" => ("Město", "City"),
        "postal_code" => ("PSČ", "Postal code"),
        "country" => ("Země", "Country"),
        "bank_account" => ("Bankovní účet", "Bank account"),
        "iban" => ("IBAN", "IBAN"),
        "archived" => ("Archivováno", "Archived"),
        _ => (key, key),
    };
    if cs {
        pair.0
    } else {
        pair.1
    }
}
#[async_trait]
impl export::ExportStore for PgExport {
    async fn export(&self, actor: &str, company: &str, request: Request) -> Result<File> {
        let cs = matches!(request.language, Language::Cs);
        let mut rows = vec![request
            .columns
            .iter()
            .map(|c| label(c, cs).to_string())
            .collect::<Vec<_>>()];
        let kind = match request.selection {
            Selection::Invoices { mut search } => {
                search.page = 1;
                search.page_size = 10001;
                let page = crate::draft::PgDraft(self.0.clone())
                    .list(actor, company, search)
                    .await?;
                if page.total > 10000 {
                    return Err(Error::Invalid(vec!["export_limit".into()]));
                }
                for d in page.items {
                    rows.push(
                        request
                            .columns
                            .iter()
                            .map(|c| match c.as_str() {
                                "number" => d.number.clone().unwrap_or_default(),
                                "description" => d
                                    .data
                                    .calculation
                                    .lines
                                    .first()
                                    .map(|l| l.description.clone())
                                    .unwrap_or_default(),
                                "status" => if d.number.is_none() {
                                    if cs {
                                        "Rozpracováno"
                                    } else {
                                        "Draft"
                                    }
                                } else {
                                    if cs {
                                        "Vystaveno"
                                    } else {
                                        "Issued"
                                    }
                                }
                                .into(),
                                "customer" => d
                                    .data
                                    .customer
                                    .as_ref()
                                    .map(|v| v.name.clone())
                                    .unwrap_or_default(),
                                "registration_number" => d
                                    .data
                                    .customer
                                    .as_ref()
                                    .map(|v| v.registration_number.clone())
                                    .unwrap_or_default(),
                                "issued_on" => date(d.data.issued_on, cs),
                                "due_on" => date(d.data.due_on, cs),
                                "currency" => d.data.calculation.currency.clone(),
                                "net" => {
                                    amount(d.totals.as_ref().map(|t| t.amounts.net.as_str()), cs)
                                }
                                "vat" => {
                                    amount(d.totals.as_ref().map(|t| t.amounts.vat.as_str()), cs)
                                }
                                "payable" => {
                                    amount(d.totals.as_ref().map(|t| t.payable.as_str()), cs)
                                }
                                "paid" => {
                                    amount(d.settlement.as_ref().map(|t| t.paid.as_str()), cs)
                                }
                                "outstanding" => amount(
                                    d.settlement.as_ref().map(|t| t.outstanding.as_str()),
                                    cs,
                                ),
                                "variable_symbol" => d.data.payment.variable_symbol.clone(),
                                _ => String::new(),
                            })
                            .collect(),
                    );
                }
                "invoices"
            }
            Selection::Contacts { mut search } => {
                search.page = 1;
                search.page_size = 10001;
                let page = crate::contact::PgContact(self.0.clone())
                    .list(actor, company, search)
                    .await?;
                if page.total > 10000 {
                    return Err(Error::Invalid(vec!["export_limit".into()]));
                }
                for c in page.items {
                    rows.push(
                        request
                            .columns
                            .iter()
                            .map(|key| match key.as_str() {
                                "name" => c.data.name.clone(),
                                "registration_number" => c.data.registration_number.clone(),
                                "vat_number" => c.data.vat_number.clone(),
                                "email" => c.data.email.clone(),
                                "phone" => c.data.phone.clone(),
                                "street" => c.data.billing_address.street.clone(),
                                "city" => c.data.billing_address.city.clone(),
                                "postal_code" => c.data.billing_address.postal_code.clone(),
                                "country" => c.data.billing_address.country.clone(),
                                "bank_account" => c.data.bank_account.clone(),
                                "iban" => c.data.iban.clone(),
                                "archived" => if c.archived {
                                    if cs {
                                        "Ano"
                                    } else {
                                        "Yes"
                                    }
                                } else {
                                    if cs {
                                        "Ne"
                                    } else {
                                        "No"
                                    }
                                }
                                .into(),
                                _ => String::new(),
                            })
                            .collect(),
                    );
                }
                "contacts"
            }
        };
        // CPU-bound encoding stays off the async executor. No database connection is held.
        let format = request.format;
        let (bytes, ext, mime) = tokio::task::spawn_blocking(move || encode(&rows, format))
            .await
            .map_err(unavailable)??;
        Ok(File {
            filename: format!("open-faktura-{kind}.{ext}"),
            media_type: mime,
            bytes,
        })
    }
}
fn date(v: Option<chrono::NaiveDate>, cs: bool) -> String {
    v.map(|d| {
        d.format(if cs { "%d.%m.%Y" } else { "%d/%m/%Y" })
            .to_string()
    })
    .unwrap_or_default()
}
fn amount(v: Option<&str>, cs: bool) -> String {
    let v = v.unwrap_or_default();
    if cs {
        v.replace('.', ",")
    } else {
        v.into()
    }
}
fn safe_csv(v: &str) -> String {
    if v.trim_start().starts_with(['=', '+', '-', '@']) || v.starts_with(['\t', '\r', '\n']) {
        format!("'{v}")
    } else {
        v.into()
    }
}
fn xml(v: &str) -> String {
    v.chars()
        .filter(|c| {
            matches!(c, '\t' | '\n' | '\r') || (*c >= ' ' && *c != '\u{fffe}' && *c != '\u{ffff}')
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
type Encoded = (Vec<u8>, &'static str, &'static str);
fn encode(rows: &[Vec<String>], format: Format) -> Result<Encoded> {
    if matches!(format, Format::Csv) {
        let mut w = csv::WriterBuilder::new()
            .delimiter(b';')
            .terminator(csv::Terminator::CRLF)
            .from_writer(vec![0xef, 0xbb, 0xbf]);
        for r in rows {
            w.write_record(r.iter().map(|v| safe_csv(v)))
                .map_err(unavailable)?;
        }
        return Ok((
            w.into_inner().map_err(unavailable)?,
            "csv",
            "text/csv; charset=utf-8",
        ));
    }
    let mut sheet=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>");
    for (i, row) in rows.iter().enumerate() {
        sheet.push_str(&format!("<row r=\"{}\">", i + 1));
        for (j, v) in row.iter().enumerate() {
            // All cells are explicit strings: preserve identifiers, exact decimals and prevent formulas.
            sheet.push_str(&format!(
                "<c r=\"{}{}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                (b'A' + j as u8) as char,
                i + 1,
                xml(v)
            ));
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let entries = [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Export" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet.as_str()),
    ];
    for (name, data) in entries {
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(unavailable)?;
        zip.write_all(data.as_bytes()).map_err(unavailable)?;
    }
    Ok((
        zip.finish().map_err(unavailable)?.into_inner(),
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Reader;
    #[test]
    fn formats_preserve_text_and_neutralize_formulas() {
        let rows = vec![
            vec!["Název".into(), "IČO".into()],
            vec![
                " =HYPERLINK(\"bad\")\nŽluťoučký; & <".into(),
                "00123456".into(),
            ],
        ];
        let (csv, _, _) = encode(&rows, Format::Csv).unwrap();
        let mut r = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(&csv[3..]);
        let record = r.records().next().unwrap().unwrap();
        assert_eq!(&record[0], format!("'{}", rows[1][0]));
        assert_eq!(&record[1], "00123456");
        let (xlsx, _, _) = encode(&rows, Format::Xlsx).unwrap();
        let mut book = calamine::Xlsx::new(Cursor::new(xlsx)).unwrap();
        let range = book.worksheet_range("Export").unwrap();
        assert_eq!(range.get_value((1, 0)).unwrap().to_string(), rows[1][0]);
        assert_eq!(range.get_value((1, 1)).unwrap().to_string(), "00123456");
        assert!(book.worksheet_formula("Export").unwrap().is_empty());
    }
}

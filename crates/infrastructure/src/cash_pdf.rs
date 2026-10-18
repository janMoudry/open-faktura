//! Render only the persisted issuer and movement snapshot; no external I/O.
use faktura_application::company::{CompanyError as Error, Result};
use faktura_domain::cash::MovementData;
use genpdf::{
    elements::{Break, Paragraph},
    fonts::{FontData, FontFamily},
    style::Style,
    Element,
};
use sqlx::{PgPool, Row};
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn text(value: &str) -> String {
    let mut out = String::new();
    let mut run = 0;
    for ch in value.chars().filter(|c| !c.is_control()) {
        if ch.is_whitespace() {
            run = 0
        } else {
            run += 1
        };
        if run > 40 {
            out.push(' ');
            run = 1;
        }
        out.push(ch);
    }
    out
}
struct Snapshot {
    document_style: Option<faktura_application::document_style::Style>,
    retail_tax: Option<Vec<faktura_domain::invoice::TaxSummary>>,
    tax: Option<faktura_domain::cash::TaxSnapshot>,
    number: String,
    data: MovementData,
    issuer: serde_json::Value,
    cancelled: bool,
    reason: Option<String>,
}
pub(crate) async fn load(
    pool: &PgPool,
    a: &str,
    c: &str,
    id: &str,
    language: &str,
) -> Result<Vec<u8>> {
    if !["cs", "en"].contains(&language) {
        return Err(Error::Invalid(vec!["language".into()]));
    }
    let mut tx = pool.begin().await.map_err(err)?;
    let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
    let id: i64 = id.parse().map_err(|_| Error::NotFound)?;
    let row = sqlx::query("SELECT * FROM cash_movement WHERE company_id=$1 AND id=$2")
        .bind(c)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    let revision: i64 = row.get("revision");
    let cached:Option<Vec<u8>>=sqlx::query_scalar("SELECT content FROM cash_pdf WHERE company_id=$1 AND movement_id=$2 AND revision=$3 AND language=$4").bind(c).bind(id).bind(revision).bind(language).fetch_optional(&mut *tx).await.map_err(err)?;
    if let Some(pdf) = cached {
        return Ok(pdf);
    }
    let snapshot = Snapshot {
        document_style: row
            .get::<Option<serde_json::Value>, _>("document_style")
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?,
        retail_tax: row
            .try_get::<Option<serde_json::Value>, _>("retail_tax")
            .unwrap_or(None)
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?,
        tax: row
            .get::<Option<serde_json::Value>, _>("tax")
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?,
        number: row.get("number"),
        data: serde_json::from_value(row.get("data")).map_err(err)?,
        issuer: row.get("issuer"),
        cancelled: row.get("cancelled"),
        reason: row.get("reason"),
    };
    let en = language == "en";
    let pdf = tokio::task::spawn_blocking(move || render(snapshot, en))
        .await
        .map_err(err)?
        .map_err(err)?;
    sqlx::query("INSERT INTO cash_pdf(company_id,movement_id,revision,language,content) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING").bind(c).bind(id).bind(revision).bind(language).bind(&pdf).execute(&mut *tx).await.map_err(err)?;
    let pdf=sqlx::query_scalar("SELECT content FROM cash_pdf WHERE company_id=$1 AND movement_id=$2 AND revision=$3 AND language=$4").bind(c).bind(id).bind(revision).bind(language).fetch_one(&mut *tx).await.map_err(err)?;
    tx.commit().await.map_err(err)?;
    Ok(pdf)
}
fn render(s: Snapshot, en: bool) -> std::result::Result<Vec<u8>, genpdf::error::Error> {
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
    let label = |cs: &str, eng: &str| if en { eng.to_string() } else { cs.to_string() };
    let incoming = !s.data.amount.starts_with('-');
    let title = if incoming {
        label("Příjmový pokladní doklad", "Cash receipt voucher")
    } else {
        label("Výdajový pokladní doklad", "Cash disbursement voucher")
    };
    doc.set_title(format!("{title} {}", s.number));
    doc.set_font_size(10);
    doc.set_line_spacing(1.4);
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(20);
    let number = s.number.clone();
    decorator.set_header(move |page| {
        Paragraph::new(format!("Open Faktura  |  {number}  |  {page}"))
            .styled(Style::new().with_font_size(8))
    });
    doc.set_page_decorator(decorator);
    if let Some(logo) = s
        .document_style
        .as_ref()
        .and_then(|style| style.logo_base64.as_deref())
    {
        doc.push(crate::invoice_pdf::style_image(logo, 40.0)?);
    }
    doc.push(Break::new(1));
    doc.push(Paragraph::new(title).styled(Style::new().bold().with_font_size(20)));
    doc.push(Paragraph::new(&s.number).styled(Style::new().with_font_size(14)));
    doc.push(Break::new(1));
    if s.cancelled {
        doc.push(
            Paragraph::new(label("STORNOVÁNO", "CANCELLED"))
                .styled(Style::new().bold().with_font_size(16)),
        );
        doc.push(Paragraph::new(text(
            s.reason.as_deref().unwrap_or_default(),
        )));
        doc.push(Break::new(1));
    }
    let issuer = |key: &str| s.issuer[key].as_str().unwrap_or_default();
    doc.push(Paragraph::new(label("Vystavitel", "Issuer")).styled(Style::new().bold()));
    for line in [
        issuer("name").to_owned(),
        issuer("street").to_owned(),
        format!("{} {}", issuer("postal_code"), issuer("city")),
        issuer("country").to_owned(),
        if issuer("registration_number").is_empty() {
            String::new()
        } else {
            format!(
                "{}: {}",
                label("IČO", "Registration no."),
                issuer("registration_number")
            )
        },
        if issuer("vat_number").is_empty() {
            String::new()
        } else {
            format!("{}: {}", label("DIČ", "VAT ID"), issuer("vat_number"))
        },
    ] {
        if !line.trim().is_empty() {
            doc.push(Paragraph::new(text(&line)));
        }
    }
    doc.push(Break::new(1));
    for (label, value) in [
        (
            label("Datum pohybu", "Payment date"),
            s.data
                .booked_on
                .format(if en { "%d/%m/%Y" } else { "%d. %m. %Y" })
                .to_string(),
        ),
        (label("Partner", "Counterparty"), s.data.counterparty),
        (label("Osoba", "Person"), s.data.person),
        (
            label("Variabilní symbol", "Payment reference"),
            s.data.variable_symbol,
        ),
        (label("Účel platby", "Payment purpose"), s.data.description),
    ] {
        doc.push(Paragraph::new(format!("{label}: {}", text(&value))));
    }
    doc.push(Break::new(1));
    if let Some(taxes) = &s.retail_tax {
        doc.push(
            Paragraph::new(label(
                "Rozpis zaúčtovaných prodejek",
                "Posted receipts breakdown",
            ))
            .styled(Style::new().bold()),
        );
        for tax in taxes {
            let money = |v: &str| {
                if en {
                    v.to_owned()
                } else {
                    v.replace('.', ",")
                }
            };
            doc.push(Paragraph::new(format!(
                "{} % | {}: {} CZK | {}: {} CZK | {}: {} CZK",
                tax.vat_percent,
                label("Základ", "Net"),
                money(&tax.amounts.net),
                label("DPH", "VAT"),
                money(&tax.amounts.vat),
                label("Celkem", "Total"),
                money(&tax.amounts.gross)
            )));
        }
    }
    if let Some(tax) = &s.tax {
        doc.push(Paragraph::new(label("Rozpis DPH", "VAT breakdown")).styled(Style::new().bold()));
        let value = |v: &str| {
            if en {
                v.to_owned()
            } else {
                v.replace('.', ",")
            }
        };
        doc.push(Paragraph::new(format!(
            "{}: {}",
            label("Datum plnění", "Tax point date"),
            tax.tax
                .tax_on
                .format(if en { "%d/%m/%Y" } else { "%d. %m. %Y" })
        )));
        doc.push(Paragraph::new(format!(
            "{}: {} %",
            label("Sazba DPH", "VAT rate"),
            tax.tax.vat_percent
        )));
        doc.push(Paragraph::new(format!(
            "{}: {} CZK",
            label("Základ daně", "Net amount"),
            value(&tax.amounts.net)
        )));
        doc.push(Paragraph::new(format!(
            "{}: {} CZK",
            label("DPH", "VAT"),
            value(&tax.amounts.vat)
        )));
        doc.push(Break::new(1));
    }
    let value = s.data.amount.trim_start_matches('-');
    let value = if en {
        value.to_owned()
    } else {
        value.replace('.', ",")
    };
    doc.push(
        Paragraph::new(format!("{}: {value} CZK", label("Částka", "Amount")))
            .styled(Style::new().bold().with_font_size(18)),
    );
    if let Some(words) = faktura_domain::amount_words::czk(
        s.data.amount.trim_start_matches('-'),
        if en { "en" } else { "cs" },
    ) {
        doc.push(Break::new(0.5));
        doc.push(Paragraph::new(format!(
            "{}: {words}",
            label("Slovy", "In words")
        )));
    }
    doc.push(Break::new(2));
    doc.push(Paragraph::new(label(
        "Vydal: ____________________     Přijal: ____________________",
        "Paid by: ____________________     Received by: ____________________",
    )));
    if let Some(style) = &s.document_style {
        if let Some(signature) = style.signature_base64.as_deref() {
            doc.push(Break::new(0.5));
            doc.push(crate::invoice_pdf::style_image(signature, 35.0)?);
        }
        let footer = if en {
            &style.footer_en
        } else {
            &style.footer_cs
        };
        if !footer.is_empty() {
            doc.push(Break::new(0.5));
            doc.push(Paragraph::new(text(footer)).styled(Style::new().with_font_size(8)));
        }
    }
    let mut bytes = vec![];
    doc.render(&mut bytes)?;
    Ok(bytes)
}

//! Print persisted posting groups without recalculation or network I/O.
use faktura_application::retail::Posting;
use genpdf::{
    elements::{Break, Paragraph},
    fonts::{FontData, FontFamily},
    style::Style,
    Element,
};
fn text(v: &str) -> String {
    let mut run = 0;
    let mut result = String::new();
    for c in v.chars().filter(|c| !c.is_control()) {
        if c.is_whitespace() {
            run = 0;
        } else {
            run += 1;
        }
        if run > 40 {
            result.push(' ');
            run = 1;
        }
        result.push(c);
    }
    result
}
pub(crate) fn render(
    d: &Posting,
    issuer: Option<&serde_json::Value>,
    language: &str,
) -> Result<Vec<u8>, genpdf::error::Error> {
    let regular = FontData::new(
        include_bytes!("../assets/NotoSans-Regular.ttf").to_vec(),
        None,
    )?;
    let bold = FontData::new(include_bytes!("../assets/NotoSans-Bold.ttf").to_vec(), None)?;
    let mut pdf = genpdf::Document::new(FontFamily {
        regular: regular.clone(),
        bold: bold.clone(),
        italic: regular,
        bold_italic: bold,
    });
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    pdf.set_page_decorator(decorator);
    pdf.set_font_size(10);
    let en = language == "en";
    let number = |v: &str| {
        if en {
            v.to_owned()
        } else {
            v.replace('.', ",")
        }
    };
    let label = |cs: &str, eng: &str| if en { eng.to_owned() } else { cs.to_owned() };
    pdf.push(
        Paragraph::new(label(
            "Souhrnné zaúčtování prodejek",
            "Sales receipt posting summary",
        ))
        .styled(Style::new().bold().with_font_size(17)),
    );
    pdf.push(Paragraph::new(format!(
        "{}: {} · {}: {}",
        label("Dávka", "Batch"),
        d.id,
        label("Datum", "Date"),
        d.booked_on
            .format(if en { "%Y-%m-%d" } else { "%d. %m. %Y" })
    )));
    if d.cancelled {
        pdf.push(Paragraph::new(label("STORNOVÁNO", "CANCELLED")).styled(Style::new().bold()));
        pdf.push(Paragraph::new(text(&d.reason)));
    }
    if let Some(issuer) = issuer {
        for key in ["name", "street", "city", "postal_code"] {
            pdf.push(Paragraph::new(text(
                issuer[key].as_str().unwrap_or_default(),
            )));
        }
        for (key, title) in [
            ("registration_number", label("IČO", "Registration no.")),
            ("vat_number", label("DIČ", "VAT ID")),
        ] {
            pdf.push(Paragraph::new(format!(
                "{title}: {}",
                text(issuer[key].as_str().unwrap_or_default())
            )));
        }
    } else {
        pdf.push(Paragraph::new(label(
            "Historické údaje vystavitele nejsou uložené.",
            "Historical issuer details were not recorded.",
        )));
    }
    for group in &d.groups {
        pdf.push(Break::new(1));
        let method = match group.method.as_str() {
            "cash" => label("Hotovost", "Cash"),
            "card" => label("Karta", "Card"),
            _ => label("Stravenka", "Voucher"),
        };
        let identifier = group
            .internal_number
            .as_deref()
            .or(group.cash_movement_id.as_deref())
            .unwrap_or("—");
        pdf.push(Paragraph::new(format!("{method} · {identifier}")).styled(Style::new().bold()));
        if let Some(t) = &group.terminal {
            pdf.push(Paragraph::new(format!(
                "{}: {} ({})",
                label("Terminál", "Terminal"),
                text(&t.name),
                text(&t.code)
            )));
        }
        if let Some(c) = &group.customer {
            pdf.push(Paragraph::new(format!(
                "{}: {}",
                label("Odběratel", "Customer"),
                text(&c.name)
            )));
        }
        pdf.push(Paragraph::new(format!(
            "{}: {}",
            label("Zdrojová ID prodejek", "Source receipt IDs"),
            group.document_ids.join(", ")
        )));
        for tax in &group.tax_summary {
            pdf.push(Paragraph::new(format!(
                "{} {} %: {} / {} / {} CZK",
                label("Základ / DPH / celkem", "Net / VAT / gross"),
                number(&tax.vat_percent),
                number(&tax.amounts.net),
                number(&tax.amounts.vat),
                number(&tax.amounts.gross)
            )));
        }
        pdf.push(Paragraph::new(format!(
            "{}: {} CZK",
            label("Zaokrouhlení", "Rounding"),
            number(&group.rounding)
        )));
        pdf.push(
            Paragraph::new(format!(
                "{}: {} CZK",
                label("Uhrazeno", "Paid"),
                number(&group.paid)
            ))
            .styled(Style::new().bold()),
        );
    }
    let mut bytes = vec![];
    pdf.render(&mut bytes)?;
    Ok(bytes)
}

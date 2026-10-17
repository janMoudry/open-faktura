use faktura_application::retail::Document;
use genpdf::{
    elements::{Break, Paragraph},
    fonts::{FontData, FontFamily},
    style::Style,
    Element,
};
pub(crate) fn render(d: &Document) -> Result<Vec<u8>, genpdf::error::Error> {
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
    let en = d.data.invoice.language == "en";
    let label = |cs: &str, en_text: &str| {
        if en {
            en_text.to_owned()
        } else {
            cs.to_owned()
        }
    };
    pdf.push(
        Paragraph::new(format!(
            "{} {}",
            label("Prodejka", "Sales receipt"),
            d.number
        ))
        .styled(Style::new().bold().with_font_size(18)),
    );
    if d.cancelled {
        pdf.push(Paragraph::new(label("STORNOVÁNO", "CANCELLED")).styled(Style::new().bold()));
    }
    let party = &d.data.invoice.supplier;
    pdf.push(Paragraph::new(party.name.clone()));
    pdf.push(Paragraph::new(format!(
        "{}, {} {}",
        party.address.street, party.address.postal_code, party.address.city
    )));
    pdf.push(Paragraph::new(format!(
        "{}: {}  {}: {}",
        label("IČO", "Registration no."),
        party.registration_number,
        label("DIČ", "VAT ID"),
        party.vat_number
    )));
    pdf.push(Paragraph::new(format!(
        "{}: {}",
        label("Datum prodeje", "Sale date"),
        d.data.invoice.issued_on.unwrap()
    )));
    if let Some(t) = &d.terminal {
        pdf.push(Paragraph::new(format!(
            "{}: {} ({})",
            label("Terminál", "Terminal"),
            t.name,
            t.code
        )));
    }
    if let Some(c) = &d.data.invoice.customer {
        pdf.push(Paragraph::new(format!(
            "{}: {}",
            label("Odběratel", "Customer"),
            c.name
        )));
    }
    pdf.push(Break::new(1));
    for (line, total) in d.data.invoice.calculation.lines.iter().zip(&d.totals.lines) {
        pdf.push(Paragraph::new(format!(
            "{} — {} {} × {} CZK; {} %; {} CZK",
            wrap(&line.description),
            line.quantity,
            line.unit,
            line.unit_price,
            line.vat_percent,
            total.amounts.gross
        )));
    }
    for t in &d.totals.tax_summary {
        pdf.push(Paragraph::new(format!(
            "{} {} %: {} / {} / {} CZK",
            label("Základ / DPH / celkem", "Net / VAT / gross"),
            t.vat_percent,
            t.amounts.net,
            t.amounts.vat,
            t.amounts.gross
        )));
    }
    pdf.push(Paragraph::new(format!(
        "{}: {} CZK",
        label("Zaokrouhlení", "Rounding"),
        d.totals.rounding
    )));
    pdf.push(
        Paragraph::new(format!(
            "{}: {} CZK",
            label("Uhrazeno celkem", "Total paid"),
            d.totals.payable
        ))
        .styled(Style::new().bold()),
    );
    for p in &d.allocations {
        let method = match p.method.as_str() {
            "cash" => label("Hotovost", "Cash"),
            "card" => label("Karta", "Card"),
            _ => label("Stravenka", "Voucher"),
        };
        pdf.push(Paragraph::new(format!("{method}: {} CZK", p.paid)));
    }
    pdf.push(Paragraph::new(wrap(&d.data.invoice.note)));
    let mut bytes = vec![];
    pdf.render(&mut bytes)?;
    Ok(bytes)
}
fn wrap(v: &str) -> String {
    let mut n = 0;
    let mut out = String::new();
    for c in v.chars() {
        if c.is_whitespace() {
            n = 0;
        } else {
            n += 1;
        }
        if n > 40 {
            out.push(' ');
            n = 1;
        }
        if !c.is_control() || c == '\n' {
            out.push(c);
        }
    }
    out
}

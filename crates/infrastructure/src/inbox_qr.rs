//! Offline QR Faktura SID 1.0 proposal, not QR payment initiation.
use faktura_application::company::{CompanyError as Error, Result};
use faktura_domain::{
    contact::Address,
    invoice::PartySnapshot,
    received::{amount, money, Data, Kind, Line},
};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
fn bad() -> Error {
    Error::Invalid(vec!["qr_unsupported".into()])
}
fn decode(v: &str) -> Result<String> {
    let mut out = Vec::new();
    let mut chars = v.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let a = chars.next().ok_or_else(bad)?;
            let b = chars.next().ok_or_else(bad)?;
            let a = (a as char).to_digit(16).ok_or_else(bad)?;
            let b = (b as char).to_digit(16).ok_or_else(bad)?;
            out.push((a * 16 + b) as u8);
        } else {
            out.push(b);
        }
    }
    let v = String::from_utf8(out).map_err(|_| bad())?;
    if v.chars().any(char::is_control) {
        return Err(bad());
    }
    Ok(v)
}
fn fields<'a>(s: &'a str, prefix: &str) -> Result<BTreeMap<&'a str, &'a str>> {
    let body = s.strip_prefix(prefix).ok_or_else(bad)?;
    let mut map = BTreeMap::new();
    for part in body.trim_end_matches('*').split('*') {
        let (k, v) = part.split_once(':').ok_or_else(bad)?;
        if k.is_empty()
            || !k
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
            || v.is_empty()
            || v.trim() != v
            || map.insert(k, v).is_some()
        {
            return Err(bad());
        }
    }
    if map.len() > 60 {
        return Err(bad());
    }
    Ok(map)
}
fn crc(bytes: &[u8]) -> u32 {
    let mut value = !0u32;
    for &b in bytes {
        value ^= b as u32;
        for _ in 0..8 {
            value = (value >> 1) ^ if value & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    !value
}
pub(super) fn parse(text: &str, today: chrono::NaiveDate) -> Result<Data> {
    if text.len() > 8192 || text.chars().any(char::is_control) {
        return Err(bad());
    }
    let mut owned;
    let raw = if text.starts_with("SPD*") {
        let payment = fields(text, "SPD*1.0*")?;
        owned = decode(payment.get("X-INV").ok_or_else(bad)?)?;
        // Shared keys are moved to the outer payment descriptor by the standard.
        let inner = fields(&owned, "SID*1.0*")?;
        let mut extra = String::new();
        for (outer, key) in [
            ("AM", "AM"),
            ("CC", "CC"),
            ("DT", "DT"),
            ("ACC", "ACC"),
            ("X-VS", "VS"),
        ] {
            if let Some(value) = payment.get(outer) {
                if inner.get(key).is_some_and(|v| v != value) {
                    return Err(bad());
                }
                if !inner.contains_key(key) {
                    extra.push_str(&format!("{key}:{value}*"));
                }
            }
        }
        if !owned.ends_with('*') {
            owned.push('*');
        }
        owned.push_str(&extra);
        owned.as_str()
    } else {
        text
    };
    let mut f = fields(raw, "SID*1.0*")?;
    if let Some(check) = f.remove("CRC32") {
        let mut canonical = String::from("SID*1.0*");
        for (k, v) in &f {
            canonical.push_str(&format!("{k}:{v}*"));
        }
        if check != format!("{:08X}", crc(canonical.as_bytes())) {
            return Err(bad());
        }
    }
    if f.get("CC").is_some_and(|v| *v != "CZK")
        || f.get("TP").is_some_and(|v| *v != "0")
        || f.get("TD").is_some_and(|v| *v != "9")
        || f.get("SA").is_some_and(|v| *v != "0")
        || f.contains_key("FX")
        || f.contains_key("FXA")
        || f.contains_key("TB2")
        || f.contains_key("T2")
    {
        return Err(bad());
    }
    let date = |k: &str| -> Result<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(f.get(k).ok_or_else(bad)?, "%Y%m%d").map_err(|_| bad())
    };
    let issued_on = date("DD")?;
    // This proposal supports current domestic rate buckets only. Historic rates
    // require explicit additional mapping, never inference from rounded amounts.
    if issued_on < "2024-01-01".parse().unwrap() {
        return Err(bad());
    }
    let number = decode(f.get("ID").ok_or_else(bad)?)?;
    if number.len() > 100 {
        return Err(bad());
    }
    let description = f
        .get("MSG")
        .map(|v| decode(v))
        .transpose()?
        .unwrap_or_default();
    let mut lines = Vec::new();
    let mut sum = Decimal::ZERO;
    for (base, tax, rate) in [("TB0", "T0", "21"), ("TB1", "T1", "12"), ("NTB", "", "0")] {
        if let Some(net) = f.get(base) {
            let net = amount(net)
                .filter(|v| *v >= Decimal::ZERO)
                .ok_or_else(bad)?;
            let vat = if tax.is_empty() {
                Decimal::ZERO
            } else {
                amount(f.get(tax).ok_or_else(bad)?)
                    .filter(|v| *v >= Decimal::ZERO)
                    .ok_or_else(bad)?
            };
            sum += net + vat;
            lines.push(Line {
                stock: None,
                description: description.clone(),
                rate: rate.into(),
                net: money(net),
                vat: money(vat),
            });
        } else if !tax.is_empty() && f.contains_key(tax) {
            return Err(bad());
        }
    }
    if lines.is_empty() {
        return Err(bad());
    }
    let total = amount(f.get("AM").ok_or_else(bad)?)
        .filter(|v| *v > Decimal::ZERO)
        .ok_or_else(bad)?;
    if (total - sum).abs() > Decimal::ONE {
        return Err(bad());
    }
    Ok(Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Kind::Invoice,
        receipt_method: None,
        supplier_contact_id: None,
        supplier: PartySnapshot {
            name: String::new(),
            registration_number: f
                .get("INI")
                .map(|v| decode(v))
                .transpose()?
                .unwrap_or_default(),
            vat_number: f
                .get("VII")
                .map(|v| decode(v))
                .transpose()?
                .unwrap_or_default(),
            address: Address {
                street: String::new(),
                city: String::new(),
                postal_code: String::new(),
                country: "CZ".into(),
            },
            email: String::new(),
            phone: String::new(),
        },
        external_number: number,
        variable_symbol: f
            .get("VS")
            .map(|v| decode(v))
            .transpose()?
            .unwrap_or_default(),
        bank_account: f
            .get("ACC")
            .map(|v| decode(v))
            .transpose()?
            .unwrap_or_default()
            .split('+')
            .next()
            .unwrap_or("")
            .into(),
        issued_on,
        received_on: today,
        due_on: if f.contains_key("DT") {
            date("DT")?
        } else {
            issued_on
        },
        tax_on: if f.contains_key("DUZP") {
            Some(date("DUZP")?)
        } else {
            None
        },
        currency: "CZK".into(),
        description,
        note: String::new(),
        lines,
        rounding: money(total - sum),
    })
}
pub(crate) fn image(bytes: &[u8], today: chrono::NaiveDate) -> Result<Data> {
    use std::io::Cursor;
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| bad())?;
    let (width, height) = reader.into_dimensions().map_err(|_| bad())?;
    if width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || u64::from(width) * u64::from(height) > 16_000_000
    {
        return Err(bad());
    }
    let image = image::load_from_memory(bytes)
        .map_err(|_| bad())?
        .to_luma8();
    let mut prepared =
        rqrr::PreparedImage::prepare_from_greyscale(width as usize, height as usize, |x, y| {
            image.get_pixel(x as u32, y as u32).0[0]
        });
    let mut found = None;
    for grid in prepared.detect_grids() {
        let Ok((_, text)) = grid.decode() else {
            continue;
        };
        if text.starts_with("SID*") || text.starts_with("SPD*") && text.contains("X-INV:") {
            if found.is_some() {
                return Err(bad());
            }
            found = Some(parse(&text, today)?);
        }
    }
    found.ok_or_else(bad)
}
#[cfg(test)]
mod tests {
    use super::*;
    const SID: &str =
        "SID*1.0*ID:SYNTH-QR*DD:20261018*AM:121.00*TB0:100.00*T0:21.00*MSG:TEST*INI:12345678*";
    #[test]
    fn decode_real_generated_png_and_bound_dimensions() {
        let code = qrcode::QrCode::new(SID).unwrap();
        let side = (code.width() + 8) * 6;
        let pixels = image::GrayImage::from_fn(side as u32, side as u32, |x, y| {
            let x = x as usize / 6;
            let y = y as usize / 6;
            image::Luma([
                if x >= 4
                    && y >= 4
                    && x < code.width() + 4
                    && y < code.width() + 4
                    && code[(x - 4, y - 4)] == qrcode::Color::Dark
                {
                    0
                } else {
                    255
                },
            ])
        });
        let mut bytes = Vec::new();
        image::DynamicImage::ImageLuma8(pixels)
            .write_to(&mut bytes, image::ImageOutputFormat::Png)
            .unwrap();
        let data = super::image(&bytes, "2026-10-18".parse().unwrap()).unwrap();
        assert_eq!(data.external_number, "SYNTH-QR");
        let mut bytes = Vec::new();
        image::DynamicImage::ImageLuma8(image::GrayImage::new(4097, 1))
            .write_to(&mut bytes, image::ImageOutputFormat::Png)
            .unwrap();
        assert!(super::image(&bytes, "2026-10-18".parse().unwrap()).is_err());
    }
    #[test]
    fn source_buckets_and_missing_supplier_require_review() {
        let d = parse(SID, "2026-10-18".parse().unwrap()).unwrap();
        assert_eq!(d.lines[0].vat, "21.00");
        assert!(d.supplier.name.is_empty());
        assert!(faktura_domain::received::totals(&d).is_err());
    }
    #[test]
    fn reject_duplicate_unsupported_mismatch_crc_and_payment_only() {
        for s in [
            SID.replace("AM:121.00", "AM:999.00"),
            format!("{SID}ID:OTHER*"),
            format!("{SID}CC:EUR*"),
            format!("{SID}CRC32:00000000*"),
            "SPD*1.0*AM:121.00*CC:CZK*".into(),
            SID.replace("TB0:100.00*T0:21.00*", ""),
        ] {
            assert!(parse(&s, "2026-10-18".parse().unwrap()).is_err(), "{s}");
        }
    }
}

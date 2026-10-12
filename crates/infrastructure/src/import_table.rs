//! Bounded spreadsheet adapter. No formulas, macros, links or lossy numeric identifiers.
use calamine::{Data, Reader, Xlsx};
use faktura_application::company::{CompanyError as Error, Result};
use serde_json::{json, Value};
use std::io::{Cursor, Read};
fn invalid() -> Error {
    Error::Invalid(vec!["file".into()])
}
pub fn parse(format: &str, bytes: &[u8], delimiter: &str) -> Result<Value> {
    if bytes.len() > 2_000_000 {
        return Err(invalid());
    }
    let rows: Vec<Vec<String>> = match format {
        "csv" => {
            let sep = match delimiter {
                ";" => b';',
                "," => b',',
                "tab" => b'\t',
                _ => return Err(invalid()),
            };
            let text = std::str::from_utf8(bytes)
                .map_err(|_| invalid())?
                .trim_start_matches('\u{feff}');
            let mut reader = csv::ReaderBuilder::new()
                .delimiter(sep)
                .has_headers(false)
                .from_reader(text.as_bytes());
            reader
                .records()
                .take(502)
                .map(|r| {
                    r.map(|r| r.iter().map(str::to_owned).collect())
                        .map_err(|_| invalid())
                })
                .collect::<Result<_>>()?
        }
        "xlsx" => {
            // Inspect actual decompressed bytes and coordinates before calamine allocates a dense range.
            let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid())?;
            if zip.len() > 100 {
                return Err(invalid());
            }
            let mut total = 0;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|_| invalid())?;
                let name = file.name().to_owned();
                if name.contains("vbaProject") || name.contains("externalLinks") {
                    return Err(invalid());
                }
                let mut data = Vec::new();
                file.by_ref()
                    .take(8_000_001)
                    .read_to_end(&mut data)
                    .map_err(|_| invalid())?;
                total += data.len();
                if total > 8_000_000 {
                    return Err(invalid());
                }
                if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
                    let mut xml = quick_xml::Reader::from_reader(data.as_slice());
                    loop {
                        match xml.read_event() {
                            Ok(
                                quick_xml::events::Event::Start(e)
                                | quick_xml::events::Event::Empty(e),
                            ) => {
                                if e.name().as_ref() == b"f" {
                                    return Err(invalid());
                                }
                                for a in e.attributes() {
                                    let a = a.map_err(|_| invalid())?;
                                    if a.key.as_ref() == b"r"
                                        || (e.name().as_ref() == b"dimension"
                                            && a.key.as_ref() == b"ref")
                                    {
                                        let s =
                                            std::str::from_utf8(&a.value).map_err(|_| invalid())?;
                                        for cell in s.split(':') {
                                            let mut col = 0u32;
                                            let mut row = String::new();
                                            for c in cell.chars() {
                                                if c.is_ascii_uppercase() {
                                                    col = col
                                                        .checked_mul(26)
                                                        .and_then(|v| v.checked_add(c as u32 - 64))
                                                        .ok_or_else(invalid)?;
                                                } else if c.is_ascii_digit() {
                                                    row.push(c);
                                                } else {
                                                    return Err(invalid());
                                                }
                                            }
                                            if col > 40
                                                || row.parse::<u32>().map_err(|_| invalid())? > 501
                                            {
                                                return Err(invalid());
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(quick_xml::events::Event::Eof) => break,
                            Err(_) => return Err(invalid()),
                            _ => {}
                        }
                    }
                }
            }
            let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(|_| invalid())?;
            if book.sheet_names().len() != 1 {
                return Err(invalid());
            }
            let name = book.sheet_names()[0].clone();
            let range = book.worksheet_range(&name).map_err(|_| invalid())?;
            if range.height() > 501 || range.width() > 40 {
                return Err(invalid());
            }
            range
                .rows()
                .map(|row| {
                    row.iter()
                        .map(|v| match v {
                            Data::Empty => Ok(String::new()),
                            Data::String(s) => Ok(s.clone()),
                            Data::Int(n) => Ok(n.to_string()),
                            Data::Float(n) if n.is_finite() && n.abs() < 1e15 => Ok(n.to_string()),
                            _ => Err(invalid()),
                        })
                        .collect()
                })
                .collect::<Result<_>>()?
        }
        _ => return Err(invalid()),
    };
    if rows.len() < 2
        || rows.len() > 501
        || rows[0].is_empty()
        || rows[0].len() > 40
        || rows
            .iter()
            .any(|r| r.len() != rows[0].len() || r.iter().any(|s| s.len() > 20000))
    {
        return Err(invalid());
    }
    let mut keys = std::collections::HashSet::new();
    if rows[0]
        .iter()
        .any(|s| s.trim().is_empty() || !keys.insert(s))
    {
        return Err(invalid());
    }
    Ok(json!({"headers":rows[0],"rows":rows[1..]}))
}

//! Money S3 native XML subset, evidenced by vendor XSD and sample (E-058).
use crate::bank_statement::{xml, Node};
use faktura_application::import::{self, Batch, Error, MoneyPrepare, Prepare, Record, Result};
use faktura_domain::{catalog::CatalogData, invoice::PriceMode};
fn bad() -> Error {
    Error::Invalid(vec!["money_xml".into()])
}
fn field<'a>(node: &'a Node, path: &str) -> Result<&'a str> {
    let mut n = node;
    for part in path.split('/') {
        let mut children = n.all(part);
        let Some(next) = children.next() else {
            return Ok("");
        };
        if children.next().is_some() {
            return Err(bad());
        }
        n = next;
    }
    // value needs a child path; caller obtains text via its parent after uniqueness check.
    Ok(node.value(path))
}
fn guid(s: &str) -> Result<String> {
    uuid::Uuid::parse_str(s.trim_matches(['{', '}']))
        .map(|v| v.to_string())
        .map_err(|_| bad())
}
pub(crate) fn parse(s: &str, currency: &str) -> Result<Vec<Record>> {
    if currency != "CZK" || s.len() > 2_000_000 || s.contains('\u{fffd}') {
        return Err(bad());
    }
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let mut reader = quick_xml::Reader::from_str(s);
    loop {
        match reader.read_event().map_err(|_| bad())? {
            quick_xml::events::Event::DocType(_) => return Err(bad()),
            quick_xml::events::Event::Decl(d) => {
                if let Some(e) = d.encoding() {
                    if !e.map_err(|_| bad())?.eq_ignore_ascii_case(b"UTF-8") {
                        return Err(bad());
                    }
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    let root = xml(s).map_err(|_| bad())?;
    if root.name != "MoneyData" || root.all("SeznamZasoba").count() != 1 {
        return Err(bad());
    }
    let list = root.at("SeznamZasoba").ok_or_else(bad)?;
    let mut records = Vec::new();
    for n in list.all("Zasoba") {
        if records.len() >= 500 || n.at("DeleteImport").is_some() {
            return Err(bad());
        }
        let key = format!(
            "{}:{}",
            guid(field(n, "Sklad/GUID")?)?,
            guid(field(n, "KmKarta/GUID")?)?
        );
        let prices = n
            .all("PC")
            .filter(|p| p.at("Hladina").is_none())
            .collect::<Vec<_>>();
        if prices.len() != 1 {
            return Err(bad());
        }
        let price = prices[0];
        let mode = match field(price, "SDPH")? {
            "1" | "true" => PriceMode::Gross,
            "0" | "false" => PriceMode::Net,
            _ => return Err(bad()),
        };
        // A nonzero discount or inherited/calculated price is not silently interpreted.
        if !matches!(field(price, "VypPrCeny")?, "" | "V")
            || !matches!(field(price, "Cena1/Sleva")?, "" | "0" | "0.0" | "0.00")
        {
            return Err(bad());
        }
        let curr = field(price, "Mena/Kod")?;
        records.push(Record::Catalog {
            key,
            data: Box::new(CatalogData {
                name: field(n, "KmKarta/Popis")?.into(),
                code: field(n, "KmKarta/Katalog")?.into(),
                barcode: field(n, "KmKarta/BarCode")?.into(),
                unit: field(n, "KmKarta/MJ")?.into(),
                quantity: "1".into(),
                unit_price: field(price, "Cena1/Cena")?.into(),
                currency: if curr.is_empty() {
                    currency.into()
                } else {
                    curr.into()
                },
                price_mode: mode,
                vat_percent: field(n, "konfigurace/SDPH_Prod")?.into(),
            }),
        });
    }
    if records.is_empty() {
        return Err(bad());
    }
    Ok(records)
}
impl crate::import::PgImport {
    pub async fn prepare_money(&self, a: &str, c: &str, input: MoneyPrepare) -> Result<Batch> {
        // Check access before parsing; the shared prepare repeats authorization under its lock.
        let mut tx = self.0.begin().await.map_err(|_| Error::Unavailable)?;
        crate::draft::authorize(&mut tx, a, c, true).await?;
        drop(tx);
        let records = parse(&input.xml, &input.domestic_currency)?;
        import::prepare(
            self,
            a,
            c,
            Prepare {
                request_id: input.request_id,
                source: "money_s3".into(),
                agenda: input.agenda,
                records,
            },
        )
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = include_str!("../../../tests/fixtures/import/money-s3-synthetic.xml");
    #[test]
    fn native_price_and_identity() {
        let r = parse(XML, "CZK").unwrap();
        assert_eq!(r.len(), 1);
        let Record::Catalog { data, .. } = &r[0] else {
            panic!()
        };
        assert_eq!(data.name, "Synthetic item & service");
        assert_eq!(data.unit_price, "750");
        assert!(matches!(data.price_mode, PriceMode::Gross));
        assert_eq!(data.quantity, "1");
    }
    #[test]
    fn rejects_ambiguity_entities_encoding_and_missing_price() {
        for s in [
            XML.replace("<PC>", "<PC><SDPH>0</SDPH>"),
            XML.replace(
                "<MoneyData>",
                "<!DOCTYPE MoneyData [<!ENTITY x 'y'>]><MoneyData>",
            ),
            XML.replace("UTF-8", "windows-1250"),
            XML.replace("<SDPH>1</SDPH>", ""),
            XML.replace("<VypPrCeny>V</VypPrCeny>", "<VypPrCeny>K</VypPrCeny>"),
        ] {
            assert!(parse(&s, "CZK").is_err());
        }
    }

    #[test]
    #[ignore = "requires explicit path to the downloaded vendor fixture"]
    fn published_vendor_fixture() {
        let path = std::env::var("MONEY_VENDOR_FIXTURE").expect("fixture path");
        let text = std::fs::read_to_string(path).unwrap();
        let rows = parse(&text, "CZK").unwrap();
        assert_eq!(rows.len(), 1);
        let Record::Catalog { data, .. } = &rows[0] else {
            panic!()
        };
        assert_eq!(data.name, "Kolečkové brusle");
        assert_eq!(data.unit_price, "750");
        assert!(matches!(data.price_mode, PriceMode::Gross));
        assert_eq!(data.vat_percent, "21");
    }
}

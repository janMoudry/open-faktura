use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    vies::*,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
const NS: &[u8] = b"urn:ec.europa.eu:taxud:vies:services:checkVat:types";
#[derive(Clone)]
pub struct Vies {
    client: reqwest::Client,
    gate: Arc<tokio::sync::Semaphore>,
    last: Arc<Mutex<Option<Instant>>>,
}
impl Default for Vies {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("VIES client"),
            gate: Arc::new(tokio::sync::Semaphore::new(2)),
            last: Arc::new(Mutex::new(None)),
        }
    }
}
#[async_trait]
impl ViesRegistry for Vies {
    async fn lookup(&self, vat_number: &str) -> Result<Observation> {
        let value = normalize(vat_number)?;
        let _permit = self.gate.try_acquire().map_err(|_| Error::Unavailable)?;
        {
            let mut last = self.last.lock().map_err(|_| Error::Unavailable)?;
            if last.is_some_and(|t| t.elapsed() < Duration::from_millis(500)) {
                return Err(Error::Unavailable);
            }
            *last = Some(Instant::now());
        }
        let body=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body><checkVat xmlns=\"urn:ec.europa.eu:taxud:vies:services:checkVat:types\"><countryCode>{}</countryCode><vatNumber>{}</vatNumber></checkVat></s:Body></s:Envelope>",&value[..2],&value[2..]);
        let mut response = self
            .client
            .post("https://ec.europa.eu/taxation_customs/vies/services/checkVatService")
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("SOAPAction", "\"\"")
            .body(body)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 262144 {
                return Err(Error::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(
            std::str::from_utf8(&bytes).map_err(|_| Error::Unavailable)?,
            &value,
        )
    }
}
pub fn parse(text: &str, vat_number: &str) -> Result<Observation> {
    let value = normalize(vat_number)?;
    if text.len() > 262144 || text.contains("<!DOCTYPE") || text.contains("<!ENTITY") {
        return Err(Error::Unavailable);
    }
    let mut reader = quick_xml::NsReader::from_str(text);
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|_| Error::Unavailable)?;
        match event {
            quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e) => {
                let local = e.local_name();
                let expected =
                    if matches!(local.as_ref(), b"Envelope" | b"Body" | b"Header" | b"Fault") {
                        b"http://schemas.xmlsoap.org/soap/envelope/".as_slice()
                    } else {
                        NS
                    };
                if !matches!(namespace,quick_xml::name::ResolveResult::Bound(ns) if ns.as_ref()==expected)
                {
                    return Err(Error::Unavailable);
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
    }
    let root = crate::bank_statement::xml(text).map_err(|_| Error::Unavailable)?;
    if root.name != "Envelope" || root.all("Body").count() != 1 {
        return Err(Error::Unavailable);
    }
    let body = root.at("Body").ok_or(Error::Unavailable)?;
    if body.at("Fault").is_some() || body.all("checkVatResponse").count() != 1 {
        return Err(Error::Unavailable);
    }
    let r = body.at("checkVatResponse").ok_or(Error::Unavailable)?;
    for name in ["countryCode", "vatNumber", "requestDate", "valid"] {
        if r.all(name).count() != 1 {
            return Err(Error::Unavailable);
        }
    }
    if r.value("countryCode") != &value[..2] || r.value("vatNumber") != &value[2..] {
        return Err(Error::Unavailable);
    }
    let date = r.value("requestDate");
    let day = date.get(..10).ok_or(Error::Unavailable)?;
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| Error::Unavailable)?;
    if date.len() > 10
        && &date[10..] != "Z"
        && chrono::DateTime::parse_from_rfc3339(&format!("{day}T00:00:00{}", &date[10..])).is_err()
    {
        return Err(Error::Unavailable);
    }
    let valid = match r.value("valid") {
        "true" | "1" => true,
        "false" | "0" => false,
        _ => return Err(Error::Unavailable),
    };
    Ok(Observation {
        vat_number: value,
        valid,
        requested_on: day.into(),
        checked_at: chrono::Utc::now().to_rfc3339(),
        name: r.value("name").into(),
        address: r.value("address").into(),
        source: "EU_VIES".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><checkVatResponse xmlns="urn:ec.europa.eu:taxud:vies:services:checkVat:types"><countryCode>DE</countryCode><vatNumber>123456789</vatNumber><requestDate>2026-10-18+02:00</requestDate><valid>true</valid><name>SYNTHETIC &amp; COMPANY</name><address>Synthetic street</address></checkVatResponse></s:Body></s:Envelope>"#;
    #[test]
    fn synthetic_valid_invalid_outage_and_identity() {
        let v = parse(XML, "DE123456789").unwrap();
        assert!(v.valid);
        assert_eq!(v.name, "SYNTHETIC & COMPANY");
        assert!(
            !parse(&XML.replace("<valid>true", "<valid>false"), "DE123456789")
                .unwrap()
                .valid
        );
        for text in [
            XML.replace("<countryCode>DE", "<countryCode>FR"),
            XML.replace("<valid>true", "<valid>unknown"),
            XML.replace(
                "<valid>true</valid>",
                "<valid>true</valid><valid>false</valid>",
            ),
            XML.replace(
                "urn:ec.europa.eu:taxud:vies:services:checkVat:types",
                "urn:wrong",
            ),
        ] {
            assert!(parse(&text, "DE123456789").is_err());
        }
        assert!(normalize("GB123456789").is_err());
        assert_eq!(normalize(" xi123456789 ").unwrap(), "XI123456789");
    }
}

//! Official RWS SOAP adapter, fixed endpoint from WSDL observed 18 October 2026.
use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    vat_registry::*,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
const ENDPOINT: &str =
    "https://mojedane.gov.cz/dpr/axis2/services/rozhraniCRPDPH.rozhraniCRPDPHSOAP";
#[derive(Clone)]
pub struct Rws {
    client: reqwest::Client,
    gate: Arc<tokio::sync::Semaphore>,
    last: Arc<Mutex<Option<Instant>>>,
}
impl Default for Rws {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("RWS HTTP client"),
            gate: Arc::new(tokio::sync::Semaphore::new(4)),
            last: Arc::new(Mutex::new(None)),
        }
    }
}
#[async_trait]
impl VatRegistry for Rws {
    async fn lookup(&self, vat_number: &str) -> Result<Observation> {
        let vat_number = normalize(vat_number)?;
        let _permit = self.gate.try_acquire().map_err(|_| Error::Unavailable)?;
        {
            let mut last = self.last.lock().map_err(|_| Error::Unavailable)?;
            if last.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
                return Err(Error::Unavailable);
            }
            *last = Some(Instant::now());
        }
        let body=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\"><s:Body><StatusNespolehlivySubjektRozsirenyV2Request xmlns=\"http://adis.mfcr.cz/rozhraniCRPDPH/\"><dic>{}</dic></StatusNespolehlivySubjektRozsirenyV2Request></s:Body></s:Envelope>",&vat_number[2..]);
        let mut response = self
            .client
            .post(ENDPOINT)
            .header("Content-Type", "text/xml; charset=utf-8")
            .header(
                "SOAPAction",
                "\"http://adis.mfcr.cz/rozhraniCRPDPH/getStatusNespolehlivySubjektRozsirenyV2\"",
            )
            .body(body)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 1048576 {
                return Err(Error::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(
            std::str::from_utf8(&bytes).map_err(|_| Error::Unavailable)?,
            &vat_number,
        )
    }
}
fn date(value: &str) -> Result<String> {
    let day = value.get(..10).ok_or(Error::Unavailable)?;
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| Error::Unavailable)?;
    let suffix = value.get(10..).ok_or(Error::Unavailable)?;
    if !suffix.is_empty()
        && suffix != "Z"
        && chrono::DateTime::parse_from_rfc3339(&format!("{day}T00:00:00{suffix}")).is_err()
    {
        return Err(Error::Unavailable);
    }
    Ok(day.to_owned())
}

pub fn parse(text: &str, vat_number: &str) -> Result<Observation> {
    if text.len() > 1048576
        || text.contains("<!DOCTYPE")
        || text.contains("<!ENTITY")
        || !text.contains("http://adis.mfcr.cz/rozhraniCRPDPH/")
    {
        return Err(Error::Unavailable);
    }
    // Resolve every element namespace before the structural mapping (prefixes are arbitrary).
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
                        b"http://adis.mfcr.cz/rozhraniCRPDPH/".as_slice()
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
    if body
        .all("StatusNespolehlivySubjektRozsirenyResponse")
        .count()
        != 1
        || body.at("Fault").is_some()
    {
        return Err(Error::Unavailable);
    }
    let result = body
        .at("StatusNespolehlivySubjektRozsirenyResponse")
        .ok_or(Error::Unavailable)?;
    if result.all("status").count() != 1 {
        return Err(Error::Unavailable);
    }
    let status = result.at("status").ok_or(Error::Unavailable)?;
    if status.attr("statusCode") != "0" || result.all("statusSubjektu").count() != 1 {
        return Err(Error::Unavailable);
    }
    let subject = result.at("statusSubjektu").ok_or(Error::Unavailable)?;
    if subject.attr("dic") != vat_number.strip_prefix("CZ").ok_or(Error::Unavailable)? {
        return Err(Error::Unavailable);
    }
    let reliability = match subject.attr("nespolehlivyPlatce") {
        "NE" => Reliability::Reliable,
        "ANO" => Reliability::Unreliable,
        "NENALEZEN" => Reliability::NotFound,
        _ => return Err(Error::Unavailable),
    };
    let subject_type = match subject.attr("typSubjektu") {
        "PLATCE_DPH" => SubjectType::VatPayer,
        "SKUPINA_DPH" => SubjectType::VatGroup,
        "IDENTIFIKOVANA_OSOBA" => SubjectType::IdentifiedPerson,
        "NESPOLEHLIVA_OSOBA" => SubjectType::UnreliablePerson,
        "NENALEZEN" => SubjectType::NotFound,
        _ => return Err(Error::Unavailable),
    };
    let mut accounts = vec![];
    if let Some(list) = subject.at("zverejneneUcty") {
        for account in list.all("ucet") {
            if accounts.len() >= 1000 {
                return Err(Error::Unavailable);
            }
            let number = if let Some(a) = account.at("standardniUcet") {
                let prefix = a.attr("predcisli");
                let digits = |value: &str, min: usize, max: usize| {
                    value.len() >= min
                        && value.len() <= max
                        && value.bytes().all(|b| b.is_ascii_digit())
                };
                if !digits(prefix, 0, 6)
                    || !digits(a.attr("cislo"), 1, 10)
                    || !digits(a.attr("kodBanky"), 4, 4)
                {
                    return Err(Error::Unavailable);
                }
                format!(
                    "{}{}{}/{}",
                    prefix,
                    if prefix.is_empty() { "" } else { "-" },
                    a.attr("cislo"),
                    a.attr("kodBanky")
                )
            } else if let Some(a) = account.at("nestandardniUcet") {
                a.attr("cislo").to_owned()
            } else {
                return Err(Error::Unavailable);
            };
            if number.is_empty() || number == "/" || number.len() > 100 {
                return Err(Error::Unavailable);
            }
            accounts.push(Account {
                number,
                published_on: date(account.attr("datumZverejneni"))?,
                ended_on: if account.attr("datumZverejneniUkonceni").is_empty() {
                    None
                } else {
                    Some(date(account.attr("datumZverejneniUkonceni"))?)
                },
            });
        }
    }
    Ok(Observation {
        vat_number: vat_number.into(),
        subject_type,
        reliability,
        generated_on: date(status.attr("odpovedGenerovana"))?,
        checked_at: chrono::Utc::now().to_rfc3339(),
        accounts,
        source: "CZ_RWS".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" xmlns:d="http://adis.mfcr.cz/rozhraniCRPDPH/"><s:Body><d:StatusNespolehlivySubjektRozsirenyResponse><d:status odpovedGenerovana="2026-10-18" statusCode="0" statusText="OK"/><d:statusSubjektu dic="12345678" typSubjektu="IDENTIFIKOVANA_OSOBA" nespolehlivyPlatce="NE"><d:zverejneneUcty><d:ucet datumZverejneni="2020-01-01"><d:standardniUcet predcisli="19" cislo="123456789" kodBanky="0800"/></d:ucet><d:ucet datumZverejneni="2021-01-01" datumZverejneniUkonceni="2025-01-01"><d:nestandardniUcet cislo="DE-SYNTHETIC"/></d:ucet></d:zverejneneUcty></d:statusSubjektu></d:StatusNespolehlivySubjektRozsirenyResponse></s:Body></s:Envelope>"#;
    #[test]
    fn synthetic_status_accounts_and_failure_are_distinct() {
        let v = parse(FIXTURE, "CZ12345678").unwrap();
        assert_eq!(v.subject_type, SubjectType::IdentifiedPerson);
        assert_eq!(v.accounts[0].number, "19-123456789/0800");
        assert_eq!(v.accounts[1].ended_on.as_deref(), Some("2025-01-01"));
        assert_eq!(
            parse(
                &FIXTURE.replace("2026-10-18", "2026-10-18+02:00"),
                "CZ12345678"
            )
            .unwrap()
            .generated_on,
            "2026-10-18"
        );
        for text in [
            FIXTURE.replace("kodBanky=\"0800\"", "kodBanky=\"bad\""),
            FIXTURE.replace("statusCode=\"0\"", "statusCode=\"2\""),
            FIXTURE.replace("IDENTIFIKOVANA_OSOBA", "UNKNOWN"),
            FIXTURE.replace("dic=\"12345678\"", "dic=\"87654321\""),
            format!("<!DOCTYPE x>{FIXTURE}"),
        ] {
            assert!(parse(&text, "CZ12345678").is_err());
        }
    }
}

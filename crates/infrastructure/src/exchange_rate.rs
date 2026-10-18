use async_trait::async_trait;
use chrono::NaiveDate;
use faktura_application::{
    company::{CompanyError as Error, Result},
    exchange_rate::{Rate, RateProvider},
};
use faktura_domain::exchange::{validate, ExchangeRate};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Cnb {
    client: reqwest::Client,
    gate: Arc<tokio::sync::Semaphore>,
    last: Arc<Mutex<Option<Instant>>>,
}
impl Default for Cnb {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("CNB client"),
            gate: Arc::new(tokio::sync::Semaphore::new(4)),
            last: Arc::new(Mutex::new(None)),
        }
    }
}
#[async_trait]
impl RateProvider for Cnb {
    async fn lookup(&self, currency: &str, date: NaiveDate) -> Result<Rate> {
        let today = chrono::Utc::now()
            .with_timezone(&chrono_tz::Europe::Prague)
            .date_naive();
        if date > today {
            return Err(Error::Invalid(vec!["date".into()]));
        }
        let _permit = self.gate.try_acquire().map_err(|_| Error::Unavailable)?;
        {
            let mut last = self.last.lock().map_err(|_| Error::Unavailable)?;
            if last.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
                return Err(Error::Unavailable);
            }
            *last = Some(Instant::now());
        }
        let mut response = self.client.get(format!("https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/denni_kurz.txt?date={}", date.format("%d.%m.%Y"))).send().await.map_err(|_| Error::Unavailable)?;
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 65536 {
                return Err(Error::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(
            std::str::from_utf8(&bytes).map_err(|_| Error::Unavailable)?,
            currency,
            date,
        )
    }
}
pub fn parse(text: &str, currency: &str, requested_on: NaiveDate) -> Result<Rate> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let header = lines.next().ok_or(Error::Unavailable)?;
    let (date, serial) = header.split_once(" #").ok_or(Error::Unavailable)?;
    if serial.parse::<u32>().is_err() {
        return Err(Error::Unavailable);
    }
    let published_on =
        NaiveDate::parse_from_str(date, "%d.%m.%Y").map_err(|_| Error::Unavailable)?;
    if published_on > requested_on
        || (requested_on - published_on).num_days() > 7
        || lines.next() != Some("země|měna|množství|kód|kurz")
    {
        return Err(Error::Unavailable);
    }
    let mut selected = None;
    for line in lines.filter(|l| !l.is_empty()) {
        let fields: Vec<_> = line.split('|').collect();
        if fields.len() != 5 {
            return Err(Error::Unavailable);
        }
        if fields[3] == currency {
            let rate = ExchangeRate {
                rate: fields[4].replace(',', "."),
                units: fields[2].into(),
            };
            if selected.is_some() || !validate(currency, Some(&rate)) {
                return Err(Error::Unavailable);
            }
            selected = Some(rate);
        }
    }
    Ok(Rate {
        currency: currency.into(),
        requested_on,
        published_on,
        source: "CNB".into(),
        exchange_rate: selected.ok_or(Error::NotFound)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = "16.10.2026 #200\nzemě|měna|množství|kód|kurz\nEMU|euro|1|EUR|25,125\nJaponsko|jen|100|JPY|15,250\n";
    #[test]
    fn synthetic_weekend_units_and_invalid_response() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
        let r = parse(SAMPLE, "JPY", date).unwrap();
        assert_eq!(r.exchange_rate.rate, "15.250");
        assert_eq!(r.exchange_rate.units, "100");
        assert_eq!(r.published_on.to_string(), "2026-10-16");
        for text in [
            SAMPLE.replace("16.10.2026", "18.10.2026"),
            SAMPLE.replace("16.10.2026", "01.10.2026"),
            SAMPLE.replace("25,125", "0"),
            format!("{SAMPLE}EMU|euro|1|EUR|25,000\n"),
            "<html>Error</html>".into(),
        ] {
            assert!(parse(&text, "EUR", date).is_err());
        }
        assert!(parse(SAMPLE, "USD", date).is_err());
    }
}

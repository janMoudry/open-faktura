use async_trait::async_trait;
use faktura_application::{
    ares::{Registry, RegistryContact},
    company::{CompanyError as Error, Result},
};
use faktura_domain::contact::Address;
use serde_json::Value;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Ares {
    client: reqwest::Client,
    // Shared per adapter, bounded upstream concurrency and request frequency.
    gate: Arc<tokio::sync::Semaphore>,
    last: Arc<Mutex<Option<Instant>>>,
}
impl Default for Ares {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("HTTP client"),
            gate: Arc::new(tokio::sync::Semaphore::new(4)),
            last: Arc::new(Mutex::new(None)),
        }
    }
}
#[async_trait]
impl Registry for Ares {
    async fn lookup(&self, ico: &str) -> Result<RegistryContact> {
        let _permit = self.gate.try_acquire().map_err(|_| Error::Unavailable)?;
        {
            let mut last = self.last.lock().map_err(|_| Error::Unavailable)?;
            if last.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
                return Err(Error::Unavailable);
            }
            *last = Some(Instant::now());
        }
        let mut response = self
            .client
            .get(format!(
                "https://ares.gov.cz/ekonomicke-subjekty-v-be/rest/ekonomicke-subjekty/{ico}"
            ))
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        if !response.status().is_success() {
            return Err(Error::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if bytes.len() + chunk.len() > 1_048_576 {
                return Err(Error::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        let value = serde_json::from_slice(&bytes).map_err(|_| Error::Unavailable)?;
        map(&value, ico)
    }
}
fn string(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().trim().to_owned()
}
fn number(v: &Value, key: &str) -> String {
    v[key].as_u64().map(|n| n.to_string()).unwrap_or_default()
}
fn map(v: &Value, ico: &str) -> Result<RegistryContact> {
    let name = string(v, "obchodniJmeno");
    if string(v, "ico") != ico || name.is_empty() || name.chars().count() > 200 {
        return Err(Error::Unavailable);
    }
    let a = &v["sidlo"];
    let house = number(a, "cisloDomovni");
    let orientation = format!(
        "{}{}",
        number(a, "cisloOrientacni"),
        string(a, "cisloOrientacniPismeno")
    );
    let house = if a["typCisloDomovni"].as_u64() == Some(2) && !house.is_empty() {
        format!("č. ev. {house}")
    } else {
        house
    };
    let numbers = if orientation.is_empty() {
        house
    } else if house.is_empty() {
        orientation
    } else {
        format!("{house}/{orientation}")
    };
    let street = string(a, "nazevUlice");
    let street = if street.is_empty() {
        string(a, "nazevCastiObce")
    } else {
        street
    };
    Ok(RegistryContact {
        registration_number: ico.into(),
        name,
        vat_number: string(v, "dic"),
        billing_address: Address {
            street: format!("{street} {numbers}").trim().to_owned(),
            city: string(a, "nazevObce"),
            postal_code: number(a, "psc"),
            country: string(a, "kodStatu"),
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_address_without_inventing_vat() {
        let v = serde_json::json!({"ico":"00006947","obchodniJmeno":"Test","sidlo":{"nazevUlice":"Letenská","cisloDomovni":525,"cisloOrientacni":15,"cisloOrientacniPismeno":"a","psc":11800,"kodStatu":"CZ","nazevObce":"Praha"}});
        let c = map(&v, "00006947").unwrap();
        assert_eq!(c.billing_address.street, "Letenská 525/15a");
        assert_eq!(c.billing_address.postal_code, "11800");
        assert!(c.vat_number.is_empty());
        assert!(map(&v, "12345678").is_err());
    }
}

#[cfg(test)]
mod live {
    use super::*;
    #[tokio::test]
    #[ignore = "manual live official ARES smoke test"]
    async fn official_registry() {
        let result = Ares::default().lookup("00006947").await.unwrap();
        assert_eq!(result.registration_number, "00006947");
        assert_eq!(result.name, "Ministerstvo financí");
        assert_eq!(result.billing_address.country, "CZ");
    }
}

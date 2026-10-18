//! EU VAT registration observation for invoice preparation; no tax-treatment decision.
use crate::company::{CompanyError as Error, CompanyStore, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
pub const COUNTRIES: &[&str] = &[
    "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR", "DE", "EL", "HU", "IE", "IT", "LV",
    "LT", "LU", "MT", "NL", "PL", "PT", "RO", "SK", "SI", "ES", "SE", "XI",
];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub vat_number: String,
    pub valid: bool,
    pub requested_on: String,
    pub checked_at: String,
    pub name: String,
    pub address: String,
    pub source: String,
}
#[async_trait]
pub trait ViesRegistry: Send + Sync {
    async fn lookup(&self, vat_number: &str) -> Result<Observation>;
}
pub fn normalize(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase();
    if value.len() < 3
        || value.len() > 22
        || !value.is_ascii()
        || !COUNTRIES.contains(&&value[..2])
        || !value[2..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'*')
    {
        return Err(Error::Invalid(vec!["vat_number".into()]));
    }
    Ok(value)
}
pub async fn lookup(
    store: &dyn CompanyStore,
    registry: &dyn ViesRegistry,
    actor: &str,
    company: &str,
    value: &str,
) -> Result<Observation> {
    store.get(actor, company).await?;
    registry.lookup(&normalize(value)?).await
}

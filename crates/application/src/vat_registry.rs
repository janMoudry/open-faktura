//! Read-only official Czech VAT registry observation; never changes contacts or tax treatment.
use crate::company::{CompanyError as Error, CompanyStore, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubjectType {
    VatPayer,
    VatGroup,
    IdentifiedPerson,
    UnreliablePerson,
    NotFound,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reliability {
    Reliable,
    Unreliable,
    NotFound,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub number: String,
    pub published_on: String,
    pub ended_on: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub vat_number: String,
    pub subject_type: SubjectType,
    pub reliability: Reliability,
    pub generated_on: String,
    pub checked_at: String,
    pub accounts: Vec<Account>,
    pub source: String,
}
#[async_trait]
pub trait VatRegistry: Send + Sync {
    async fn lookup(&self, vat_number: &str) -> Result<Observation>;
}
pub fn normalize(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_uppercase();
    if !value.starts_with("CZ")
        || !(3..=12).contains(&value.len())
        || !value[2..].bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error::Invalid(vec!["vat_number".into()]));
    }
    Ok(value)
}
pub async fn lookup(
    store: &dyn CompanyStore,
    registry: &dyn VatRegistry,
    actor: &str,
    company: &str,
    value: &str,
) -> Result<Observation> {
    store.get(actor, company).await?;
    registry.lookup(&normalize(value)?).await
}

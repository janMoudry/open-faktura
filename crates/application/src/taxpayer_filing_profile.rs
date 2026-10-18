//! Explicit EPO taxpayer metadata, independent of invoice VAT calculation mode.
use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub tax_office: String,
    pub tax_office_workplace: Option<String>,
    pub taxpayer_number: String,
    pub taxpayer_kind: String,
    pub vat_filer_kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub revision: i64,
    pub data: Option<Data>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    pub data: Data,
}
pub fn validate(v: &Command) -> Result<()> {
    let digits = |s: &str, max: usize| {
        !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_digit())
    };
    if uuid::Uuid::parse_str(&v.request_id).is_err()
        || !(0..=1_000_000_000_000).contains(&v.revision)
        || !digits(&v.data.tax_office, 3)
        || v.data.tax_office.bytes().all(|b| b == b'0')
        || v.data
            .tax_office_workplace
            .as_ref()
            .is_some_and(|s| !digits(s, 4))
        || !digits(&v.data.taxpayer_number, 10)
        || !["F", "P"].contains(&v.data.taxpayer_kind.as_str())
        || !["P", "I", "S", "N", "R", "D"].contains(&v.data.vat_filer_kind.as_str())
    {
        return Err(Error::Invalid(vec!["taxpayer_profile".into()]));
    }
    let catalogue: serde_json::Value =
        serde_json::from_str(include_str!("../assets/epo-offices.json"))
            .map_err(|_| Error::Unavailable)?;
    if !catalogue["offices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|o| o["code"].as_str() == Some(&v.data.tax_office))
        || v.data.tax_office_workplace.as_ref().is_some_and(|code| {
            !catalogue["workplaces"].as_array().unwrap().iter().any(|o| {
                o["code"].as_str() == Some(code) && o["office"].as_str() == Some(&v.data.tax_office)
            })
        })
    {
        return Err(Error::Invalid(vec!["taxpayer_profile".into()]));
    }
    Ok(())
}
#[async_trait]
pub trait TaxpayerFilingProfileStore: Send + Sync {
    async fn get(&self, actor: &str, company: &str) -> Result<Profile>;
    async fn set(&self, actor: &str, company: &str, input: Command) -> Result<Profile>;
}

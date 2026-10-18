//! Explicit taxpayer report inputs, kept separate from recorded VAT and legal conclusions.
use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub kind: String,
    pub id: String,
    pub number: String,
    pub side: String,
    pub vat_on: Option<chrono::NaiveDate>,
    pub turnover_on: Option<chrono::NaiveDate>,
    pub recorded_vat_czk: Option<String>,
    pub turnover_net_czk: Option<String>,
    pub fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub turnover_included: Option<bool>,
    pub claimed_input_vat_czk: Option<String>,
    pub claim_on: Option<chrono::NaiveDate>,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Classification {
    pub revision: i64,
    pub observation: Observation,
    pub data: Data,
    pub current: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub observation: Observation,
    pub classification: Option<Classification>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    pub fingerprint: String,
    pub data: Data,
}
#[derive(Serialize, Deserialize)]
pub struct Summary {
    pub year: i32,
    pub included_turnover_czk: String,
    pub claimed_input_vat_czk: String,
    pub unclassified_turnover: i64,
    pub unclassified_input: i64,
    pub outdated: i64,
    pub classified: i64,
}
#[async_trait]
pub trait TaxClassificationStore: Send + Sync {
    async fn get(&self, a: &str, c: &str, kind: &str, id: &str) -> Result<Source>;
    async fn set(
        &self,
        a: &str,
        c: &str,
        kind: &str,
        id: &str,
        input: Command,
    ) -> Result<Classification>;
    async fn summary(&self, a: &str, c: &str, year: i32) -> Result<Summary>;
}
pub fn validate(kind: &str, id: &str, input: Option<&Command>) -> Result<()> {
    if ![
        "invoice",
        "credit_note",
        "tax_receipt",
        "retail",
        "received",
        "purchase_receipt",
        "cash",
    ]
    .contains(&kind)
        || id.parse::<i64>().map_or(true, |id| id <= 0)
    {
        return Err(Error::NotFound);
    }
    if let Some(v) = input {
        if uuid::Uuid::parse_str(&v.request_id).is_err()
            || !(0..=1_000_000_000_000).contains(&v.revision)
            || v.fingerprint.len() != 64
            || !v.fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
            || v.data.reason.trim().is_empty()
            || v.data.reason.chars().count() > 1000
            || v.data.reason.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(Error::Invalid(vec!["tax_classification".into()]));
        }
    }
    Ok(())
}

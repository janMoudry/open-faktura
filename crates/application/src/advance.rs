use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub request_id: String,
    pub revision: i64,
    pub payment: faktura_domain::payment::PaymentInput,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reverse {
    pub revision: i64,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
pub struct Receipt {
    pub id: String,
    pub amount: String,
    pub paid_on: String,
    pub method: String,
    pub note: String,
    pub reversed: bool,
    pub reason: Option<String>,
    pub tax_receipt_id: Option<String>,
    pub tax_receipt_number: Option<String>,
}
#[async_trait]
pub trait AdvanceStore: Send + Sync {
    async fn pdf(&self, a: &str, c: &str, d: &str) -> Result<Vec<u8>>;
    async fn payments(&self, a: &str, c: &str, d: &str) -> Result<Vec<Receipt>>;
    async fn register(&self, a: &str, c: &str, d: &str, cmd: Register) -> Result<()>;
    async fn reverse(&self, a: &str, c: &str, d: &str, p: &str, cmd: Reverse) -> Result<()>;
}
pub async fn register(
    s: &dyn AdvanceStore,
    a: &str,
    c: &str,
    d: &str,
    cmd: Register,
) -> Result<()> {
    let mut errors = cmd.payment.validate();
    if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
        errors.push("request_id".into());
    }
    if cmd.revision < 1 {
        errors.push("revision".into());
    }
    if !("1900-01-01"..="9999-12-31").contains(&cmd.payment.paid_on.to_string().as_str()) {
        errors.push("paid_on".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    s.register(a, c, d, cmd).await
}
pub async fn reverse(
    s: &dyn AdvanceStore,
    a: &str,
    c: &str,
    d: &str,
    p: &str,
    mut cmd: Reverse,
) -> Result<()> {
    cmd.reason = cmd.reason.trim().into();
    if cmd.revision < 1
        || cmd.reason.is_empty()
        || cmd.reason.len() > 500
        || cmd.reason.chars().any(char::is_control)
    {
        return Err(Error::Invalid(vec!["reason".into()]));
    }
    s.reverse(a, c, d, p, cmd).await
}

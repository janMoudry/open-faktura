pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
pub use faktura_domain::payment::PaymentInput;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settlement {
    pub paid: String,
    pub outstanding: String,
    pub overpayment: String,
    pub status: String,
    pub overdue: bool,
    pub revision: i64,
    pub today: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub amount: String,
    pub paid_on: String,
    pub method: String,
    pub note: String,
    pub actor_id: String,
    pub created_at: String,
    pub reversed_at: Option<String>,
    pub reason: Option<String>,
    pub reversed_by: Option<String>,
    pub replacement_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Payments {
    pub summary: Settlement,
    pub items: Vec<Payment>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    pub request_id: String,
    pub revision: i64,
    pub payment: PaymentInput,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reverse {
    pub request_id: String,
    pub revision: i64,
    pub reason: String,
    pub replacement: Option<PaymentInput>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Overview {
    pub issued_count: i64,
    pub draft_count: i64,
    pub outstanding_count: i64,
    pub overdue_count: i64,
    pub outstanding: String,
    pub overdue: String,
    pub received: String,
    pub overpayment: String,
    pub currency: String,
    pub today: String,
}
#[async_trait]
pub trait PaymentStore: Send + Sync {
    async fn get(&self, actor: &str, company: &str, invoice: &str) -> Result<Payments>;
    async fn register(
        &self,
        actor: &str,
        company: &str,
        invoice: &str,
        command: Register,
    ) -> Result<Payments>;
    async fn reverse(
        &self,
        actor: &str,
        company: &str,
        invoice: &str,
        payment: &str,
        command: Reverse,
    ) -> Result<Payments>;
    async fn overview(&self, actor: &str, company: &str) -> Result<Overview>;
}
pub async fn get(s: &dyn PaymentStore, a: &str, c: &str, i: &str) -> Result<Payments> {
    s.get(a, c, i).await
}
pub async fn overview(s: &dyn PaymentStore, a: &str, c: &str) -> Result<Overview> {
    s.overview(a, c).await
}
pub async fn register(
    s: &dyn PaymentStore,
    a: &str,
    c: &str,
    i: &str,
    cmd: Register,
) -> Result<Payments> {
    let mut e = cmd.payment.validate();
    if uuid::Uuid::parse_str(&cmd.request_id).is_err() || cmd.revision < 0 {
        e.push("request".into());
    }
    if !e.is_empty() {
        return Err(Error::Invalid(e));
    }
    s.register(a, c, i, cmd).await
}
pub async fn reverse(
    s: &dyn PaymentStore,
    a: &str,
    c: &str,
    i: &str,
    p: &str,
    cmd: Reverse,
) -> Result<Payments> {
    let mut e = cmd
        .replacement
        .as_ref()
        .map(|p| p.validate())
        .unwrap_or_default();
    if cmd.reason.trim().is_empty() || cmd.reason.chars().count() > 500 {
        e.push("reason".into());
    }
    if uuid::Uuid::parse_str(&cmd.request_id).is_err() || cmd.revision < 0 {
        e.push("request".into());
    }
    if !e.is_empty() {
        return Err(Error::Invalid(e));
    }
    s.reverse(a, c, i, p, cmd).await
}

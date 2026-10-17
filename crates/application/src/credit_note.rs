use crate::{
    company::{CompanyError as Error, Result},
    draft::Draft,
};
use async_trait::async_trait;
pub use faktura_domain::credit_note::Data;
use faktura_domain::invoice::Totals;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub kind: String,
    pub amount: String,
    pub date: String,
    pub method: String,
    pub reason: String,
    pub reverses: Option<String>,
    pub actor_id: String,
    pub created_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub company_id: String,
    pub invoice_id: String,
    pub invoice_number: String,
    pub number: Option<String>,
    pub status: String,
    pub revision: i64,
    pub source: Draft,
    pub data: Data,
    pub totals: Totals,
    pub remaining: String,
    pub offset: String,
    pub refunded: String,
    pub events: Vec<Event>,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Document>,
    pub total: i64,
    pub page: i64,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub query: String,
    pub invoice_id: Option<String>,
    pub page: Option<i64>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub invoice_id: String,
}
#[derive(Deserialize, Serialize)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Update {
        data: Data,
    },
    Issue,
    Cancel,
    Offset,
    Refund {
        amount: String,
        date: chrono::NaiveDate,
        method: String,
        reason: String,
    },
    Reverse {
        event_id: String,
        reason: String,
    },
}
#[async_trait]
pub trait CreditStore: Send + Sync {
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Document>;
    async fn command(&self, a: &str, c: &str, id: &str, cmd: Command) -> Result<Document>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Document>;
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn pdf(&self, a: &str, c: &str, id: &str) -> Result<Vec<u8>>;
}
pub async fn create(s: &dyn CreditStore, a: &str, c: &str, cmd: Create) -> Result<Document> {
    if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
        return Err(Error::Invalid(vec!["request".into()]));
    }
    s.create(a, c, cmd).await
}
pub async fn command(
    s: &dyn CreditStore,
    a: &str,
    c: &str,
    id: &str,
    cmd: Command,
) -> Result<Document> {
    let text = |v: &str| {
        !v.trim().is_empty()
            && v.chars().count() <= 1000
            && !v.chars().any(|c| c.is_control() && c != '\n')
    };
    let invalid = match &cmd.action {
        Action::Update { data } => {
            data.lines.len() > 500
                || data.reason.chars().count() > 1000
                || !["cs", "en"].contains(&data.language.as_str())
                || data.reason.chars().any(|c| c.is_control() && c != '\n')
        }
        Action::Refund {
            amount,
            method,
            reason,
            date,
        } => {
            !text(reason)
                || !["cash", "bank_transfer"].contains(&method.as_str())
                || !faktura_domain::payment::PaymentInput {
                    amount: amount.clone(),
                    paid_on: *date,
                    method: method.clone(),
                    note: String::new(),
                }
                .validate()
                .is_empty()
        }
        Action::Reverse { reason, .. } => !text(reason),
        _ => false,
    };
    if invalid || cmd.revision < 1 || uuid::Uuid::parse_str(&cmd.request_id).is_err() {
        return Err(Error::Invalid(vec!["credit_note".into()]));
    }
    s.command(a, c, id, cmd).await
}
pub async fn list(s: &dyn CreditStore, a: &str, c: &str, q: Search) -> Result<Page> {
    if q.query.len() > 200 || !(1..=10000).contains(&q.page.unwrap_or(1)) {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    s.list(a, c, q).await
}

pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
pub use faktura_domain::received::{Data, Payment, Totals};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub company_id: String,
    pub number: String,
    pub revision: i64,
    pub cancelled: bool,
    pub data: Data,
    pub totals: Totals,
    pub paid: String,
    pub remaining: String,
    pub payment_status: String,
    pub payments: Vec<Payment>,
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
    pub tag_id: String,
    pub kind: faktura_domain::received::Kind,
    pub query: String,
    pub status: String,
    pub page: Option<i64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub data: Data,
}
#[derive(Serialize, Deserialize)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Update {
        data: Box<Data>,
    },
    Pay {
        payment: faktura_domain::payment::PaymentInput,
    },
    Reverse {
        payment_id: String,
        reason: String,
    },
    Cancel {
        reason: String,
    },
}
#[async_trait]
pub trait ReceivedStore: Send + Sync {
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Document>;
    async fn command(&self, a: &str, c: &str, id: &str, cmd: Command) -> Result<Document>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Document>;
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn preview(&self, a: &str, c: &str, data: Data) -> Result<Totals>;
}
pub async fn create(s: &dyn ReceivedStore, a: &str, c: &str, cmd: Create) -> Result<Document> {
    key(&cmd.request_id)?;
    faktura_domain::received::totals(&cmd.data).map_err(Error::Invalid)?;
    s.create(a, c, cmd).await
}
fn key(v: &str) -> Result<()> {
    uuid::Uuid::parse_str(v)
        .map(|_| ())
        .map_err(|_| Error::Invalid(vec!["request_id".into()]))
}
pub async fn command(
    s: &dyn ReceivedStore,
    a: &str,
    c: &str,
    id: &str,
    cmd: Command,
) -> Result<Document> {
    key(&cmd.request_id)?;
    if cmd.revision < 1 {
        return Err(Error::Invalid(vec!["revision".into()]));
    }
    match &cmd.action {
        Action::Update { data } => {
            faktura_domain::received::totals(data).map_err(Error::Invalid)?;
        }
        Action::Pay { payment } => {
            let e = payment.validate();
            if !e.is_empty() {
                return Err(Error::Invalid(e));
            }
        }
        Action::Reverse { reason, .. } | Action::Cancel { reason } => {
            if reason.trim().is_empty()
                || reason.chars().count() > 500
                || reason.chars().any(char::is_control)
            {
                return Err(Error::Invalid(vec!["reason".into()]));
            }
        }
    };
    s.command(a, c, id, cmd).await
}

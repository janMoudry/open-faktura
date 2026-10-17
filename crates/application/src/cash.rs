pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
pub use faktura_domain::cash::{AccountData, MovementData, TaxInput, TaxRequest, TaxSnapshot};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub revision: i64,
    pub data: AccountData,
    pub archived: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    pub target_kind: String,
    pub document_id: String,
    pub document_number: String,
    pub document_kind: String,
    pub payment_id: String,
    pub created_payment: bool,
    pub created_at: String,
    pub actor_id: String,
    pub detached_at: Option<String>,
    pub detached_by: Option<String>,
    pub reason: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Movement {
    #[serde(default)]
    pub retail_posting_id: Option<String>,
    #[serde(default)]
    pub tax: Option<Box<TaxSnapshot>>,
    #[serde(default)]
    pub automatic: bool,
    pub id: String,
    pub account_id: String,
    pub number: String,
    pub revision: i64,
    pub cancelled: bool,
    pub reason: Option<String>,
    pub data: MovementData,
    pub matches: Vec<Match>,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Movement>,
    pub total: i64,
    pub page: i64,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub account_id: String,
    pub direction: String,
    pub status: String,
    pub query: String,
    pub page: Option<i64>,
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
}
pub use crate::banking::Candidate;
#[derive(Clone, Serialize, Deserialize)]
pub struct Balance {
    pub opening: String,
    pub income: String,
    pub expense: String,
    pub closing: String,
    pub count: i64,
}
#[derive(Serialize, Deserialize)]
pub struct Command {
    pub request_id: String,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Terminal {
        id: Option<String>,
        revision: Option<i64>,
        code: String,
        name: String,
        archived: bool,
    },
    TaxManual {
        account_id: String,
        data: MovementData,
        tax: TaxInput,
    },
    Settings {
        account_id: Option<String>,
        revision: i64,
    },
    Account {
        id: Option<String>,
        revision: Option<i64>,
        data: AccountData,
        archived: bool,
    },
    Manual {
        account_id: String,
        data: MovementData,
    },
    Match {
        movement_id: String,
        revision: i64,
        target_kind: String,
        document_id: String,
        target_revision: i64,
        payment_id: Option<String>,
    },
    Unmatch {
        movement_id: String,
        revision: i64,
        reason: String,
    },
    Cancel {
        movement_id: String,
        revision: i64,
        reason: String,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Reply {
    Terminal(Terminal),
    Settings(Settings),
    Account(Account),
    Movement(Movement),
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub account_id: Option<String>,
    pub revision: i64,
}
#[async_trait]
pub trait CashStore: Send + Sync {
    async fn terminals(&self, a: &str, c: &str) -> Result<Vec<Terminal>>;
    async fn tax_preview(&self, a: &str, c: &str, request: TaxRequest) -> Result<TaxSnapshot>;
    async fn settings(&self, a: &str, c: &str) -> Result<Settings>;
    async fn pdf(&self, a: &str, c: &str, id: &str, language: &str) -> Result<Vec<u8>>;
    async fn accounts(&self, a: &str, c: &str) -> Result<Vec<Account>>;
    async fn movements(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn balance(&self, a: &str, c: &str, q: Search) -> Result<Balance>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Movement>;
    async fn candidates(&self, a: &str, c: &str, id: &str, query: &str) -> Result<Vec<Candidate>>;
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Reply>;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Terminal {
    pub id: String,
    pub revision: i64,
    pub code: String,
    pub name: String,
    pub archived: bool,
}

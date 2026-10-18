pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
pub use faktura_domain::banking::{AccountData, MovementData};
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
    #[serde(default)]
    pub foreign: Option<ForeignSettlement>,
    pub amount: String,
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
fn czk_currency() -> String {
    "CZK".into()
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ForeignSettlement {
    #[serde(default = "czk_currency")]
    pub bank_currency: String,
    #[serde(default)]
    pub bank_rate: Option<faktura_domain::exchange::ExchangeRate>,
    #[serde(default)]
    pub bank_value_czk: Option<String>,
    pub currency: String,
    pub document_amount: String,
    pub document_rate: faktura_domain::exchange::ExchangeRate,
    pub book_value_czk: String,
    pub difference_czk: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ForeignCandidate {
    pub payment_id: Option<String>,
    pub payment_amount: Option<String>,
    pub target_kind: String,
    pub document_id: String,
    pub number: String,
    pub partner: String,
    pub revision: i64,
    pub outstanding: String,
    pub currency: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Movement {
    pub allocated: String,
    pub remaining: String,
    pub id: String,
    pub account_id: String,
    pub number: String,
    pub revision: i64,
    pub source: String,
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
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PreviewRow {
    pub row: usize,
    pub data: MovementData,
    pub status: String,
    pub fields: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub rows: Vec<PreviewRow>,
    pub new_count: usize,
    pub duplicate_count: usize,
    pub valid: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub available: String,
    pub target_kind: String,
    pub document_kind: String,
    pub document_id: String,
    pub number: String,
    pub partner: String,
    pub revision: i64,
    pub payment_id: Option<String>,
    pub outstanding: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileInput {
    pub account_id: String,
    pub csv: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementInput {
    pub account_id: String,
    pub format: String,
    pub filename: String,
    pub content_base64: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchInput {
    pub movement_ids: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct BatchItem {
    pub movement: Movement,
    pub candidate: Option<Candidate>,
    pub status: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct BatchPreview {
    pub items: Vec<BatchItem>,
    pub ready_count: usize,
    pub fingerprint: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allocation {
    pub target_kind: String,
    pub document_id: String,
    pub target_revision: i64,
    pub payment_id: Option<String>,
    pub amount: String,
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
    ForeignMatch {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bank_rate: Option<faktura_domain::exchange::ExchangeRate>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payment_id: Option<String>,
        movement_id: String,
        revision: i64,
        target_kind: String,
        document_id: String,
        target_revision: i64,
        bank_amount: String,
        document_amount: String,
    },
    Opening {
        account_id: String,
        revision: i64,
        opening_on: chrono::NaiveDate,
        opening_balance: String,
        reason: String,
    },
    Account {
        id: Option<String>,
        revision: Option<i64>,
        data: AccountData,
        archived: bool,
    },
    Import {
        account_id: String,
        csv: String,
    },
    Statement {
        #[serde(flatten)]
        file: StatementInput,
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
    Split {
        movement_id: String,
        revision: i64,
        allocations: Vec<Allocation>,
    },
    BatchMatch {
        movement_ids: Vec<String>,
        fingerprint: String,
    },
    Unmatch {
        movement_id: String,
        revision: i64,
        reason: String,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
// Short-lived command response; keep the public movement payload unboxed.
#[allow(clippy::large_enum_variant)]
pub enum Reply {
    Account(Account),
    Movement(Movement),
    Import(ImportPreview),
    Batch(Vec<Movement>),
}
#[async_trait]
pub trait BankStore: Send + Sync {
    async fn foreign_report(
        &self,
        a: &str,
        c: &str,
        q: ForeignReportQuery,
    ) -> Result<ForeignReport>;
    async fn foreign_candidates(
        &self,
        a: &str,
        c: &str,
        m: &str,
        q: &str,
    ) -> Result<Vec<ForeignCandidate>>;
    async fn balance(&self, a: &str, c: &str, q: BalanceQuery) -> Result<Balance>;
    async fn accounts(&self, a: &str, c: &str) -> Result<Vec<Account>>;
    async fn movements(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Movement>;
    async fn preview(&self, a: &str, c: &str, f: FileInput) -> Result<ImportPreview>;
    async fn statement_preview(&self, a: &str, c: &str, f: StatementInput)
        -> Result<ImportPreview>;
    async fn candidates(&self, a: &str, c: &str, id: &str, query: &str) -> Result<Vec<Candidate>>;
    async fn split_candidates(
        &self,
        a: &str,
        c: &str,
        id: &str,
        query: &str,
    ) -> Result<Vec<Candidate>>;
    async fn suggestions(&self, a: &str, c: &str, id: &str) -> Result<Vec<Candidate>>;
    async fn batch_preview(&self, a: &str, c: &str, input: BatchInput) -> Result<BatchPreview>;
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Reply>;
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BalanceQuery {
    pub account_id: String,
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
}
#[derive(Serialize)]
pub struct Balance {
    pub currency: String,
    pub opening_on: Option<chrono::NaiveDate>,
    pub opening_balance: Option<String>,
    pub opening: Option<String>,
    pub income: String,
    pub expense: String,
    pub closing: Option<String>,
    pub count: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForeignReportQuery {
    pub from: chrono::NaiveDate,
    pub to: chrono::NaiveDate,
    pub page: Option<i64>,
}
#[derive(Serialize)]
pub struct ForeignReportEntry {
    pub id: String,
    pub movement_id: String,
    pub movement_number: String,
    pub document_id: String,
    pub document_number: String,
    pub target_kind: String,
    pub booked_on: chrono::NaiveDate,
    pub bank_amount: String,
    pub settlement: ForeignSettlement,
}
#[derive(Serialize)]
pub struct ForeignReport {
    pub gains: String,
    pub losses: String,
    pub net: String,
    pub currency: String,
    pub total: i64,
    pub page: i64,
    pub items: Vec<ForeignReportEntry>,
}

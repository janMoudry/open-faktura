use crate::company::Result;
use async_trait::async_trait;
pub use faktura_domain::retail::{Allocation, Data, Preview};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    #[serde(default)]
    pub posting_id: Option<String>,
    pub id: String,
    pub number: String,
    pub revision: i64,
    pub data: Data,
    pub totals: faktura_domain::invoice::Totals,
    pub allocations: Vec<Allocation>,
    pub terminal: Option<crate::cash::Terminal>,
    pub cancelled: bool,
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
    pub page: Option<i64>,
    pub cancelled: bool,
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
    Save {
        id: Option<String>,
        revision: Option<i64>,
        data: Box<Data>,
        reason: String,
    },
    Cancel {
        id: String,
        revision: i64,
        reason: String,
    },
}
#[derive(Serialize)]
pub struct Audit {
    pub revision: i64,
    pub reason: String,
    pub actor_id: String,
    pub created_at: String,
}
#[async_trait]
pub trait RetailStore: Send + Sync {
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Document>;
    async fn preview(&self, a: &str, c: &str, data: Data) -> Result<Preview>;
    async fn execute(&self, a: &str, c: &str, cmd: Command) -> Result<Document>;
    async fn pdf(&self, a: &str, c: &str, id: &str) -> Result<Vec<u8>>;
    async fn history(&self, a: &str, c: &str, id: &str) -> Result<Vec<Audit>>;
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostingInput {
    pub document_ids: Vec<String>,
    pub booked_on: chrono::NaiveDate,
    pub cash_account_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PostingGroup {
    pub method: String,
    pub terminal: Option<crate::cash::Terminal>,
    pub customer: Option<faktura_domain::invoice::PartySnapshot>,
    pub document_ids: Vec<String>,
    pub paid: String,
    pub rounding: String,
    pub tax_summary: Vec<faktura_domain::invoice::TaxSummary>,
    pub cash_movement_id: Option<String>,
    pub internal_number: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PostingPreview {
    pub groups: Vec<PostingGroup>,
    pub fingerprint: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Posting {
    pub id: String,
    pub revision: i64,
    pub booked_on: chrono::NaiveDate,
    pub groups: Vec<PostingGroup>,
    pub cancelled: bool,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostCommand {
    pub request_id: String,
    pub input: PostingInput,
    pub fingerprint: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnpostCommand {
    pub request_id: String,
    pub id: String,
    pub revision: i64,
    pub reason: String,
}
#[async_trait]
pub trait PostingStore: Send + Sync {
    async fn posting_preview(
        &self,
        a: &str,
        c: &str,
        input: PostingInput,
    ) -> Result<PostingPreview>;
    async fn post(&self, a: &str, c: &str, cmd: PostCommand) -> Result<Posting>;
    async fn unpost(&self, a: &str, c: &str, cmd: UnpostCommand) -> Result<Posting>;
    async fn postings(&self, a: &str, c: &str) -> Result<Vec<Posting>>;
}

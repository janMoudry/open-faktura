use crate::{
    company::{CompanyError as Error, Result},
    draft::Draft,
    sales::Document,
};
use async_trait::async_trait;
use faktura_domain::invoice::{Calculation, Totals};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub revision: i64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configure {
    pub request_id: String,
    pub invoice_id: Option<String>,
    pub revision: Option<i64>,
    pub advances: Vec<Selection>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub query: String,
    pub page: Option<i64>,
}
#[derive(Serialize)]
pub struct Candidates {
    pub items: Vec<Document>,
    pub total: i64,
    pub page: i64,
}
#[async_trait]
pub trait SettlementStore: Send + Sync {
    async fn configure(&self, a: &str, c: &str, cmd: Configure) -> Result<Draft>;
    async fn candidates(&self, a: &str, c: &str, q: Search) -> Result<Candidates>;
    async fn preview(&self, a: &str, c: &str, id: &str, calculation: Calculation)
        -> Result<Totals>;
}
pub async fn configure(s: &dyn SettlementStore, a: &str, c: &str, cmd: Configure) -> Result<Draft> {
    let mut ids = std::collections::HashSet::new();
    if uuid::Uuid::parse_str(&cmd.request_id).is_err()
        || cmd.advances.len() > 20
        || (cmd.invoice_id.is_none() && (cmd.advances.is_empty() || cmd.revision.is_some()))
        || (cmd.invoice_id.is_some() && cmd.revision.is_none_or(|v| v < 1))
        || cmd
            .advances
            .iter()
            .any(|v| v.revision < 1 || !ids.insert(&v.id))
    {
        return Err(Error::Invalid(vec!["advance_selection".into()]));
    }
    s.configure(a, c, cmd).await
}
pub async fn candidates(
    s: &dyn SettlementStore,
    a: &str,
    c: &str,
    q: Search,
) -> Result<Candidates> {
    if q.query.len() > 200 || !(1..=10000).contains(&q.page.unwrap_or(1)) {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    s.candidates(a, c, q).await
}

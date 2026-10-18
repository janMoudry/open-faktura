pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use faktura_domain::invoice::{DraftData, Totals};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    #[serde(default)]
    pub advances: Option<faktura_domain::advance_settlement::Settlement>,
    pub id: String,
    pub company_id: String,
    pub revision: i64,
    pub data: DraftData,
    pub totals: Option<Totals>,
    pub number: Option<String>,
    pub settlement: Option<crate::payment::Settlement>,
    pub issued_at: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct DraftPage {
    pub items: Vec<Draft>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub status: String,
    pub query: String,
    pub page: i64,
    pub page_size: i64,
    pub sort: String,
    pub customer_id: Option<String>,
    pub date_field: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub amount_min: Option<String>,
    pub amount_max: Option<String>,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            status: "all".into(),
            query: String::new(),
            page: 1,
            page_size: 25,
            sort: "updated_desc".into(),
            customer_id: None,
            date_field: "issued_on".into(),
            date_from: None,
            date_to: None,
            amount_min: None,
            amount_max: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub data: DraftData,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub revision: i64,
    pub data: DraftData,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub request_id: String,
    pub revision: i64,
}
#[async_trait]
pub trait DraftStore: Send + Sync {
    async fn issue(&self, actor: &str, company: &str, id: &str, command: Issue) -> Result<Draft>;
    async fn pdf(&self, actor: &str, company: &str, id: &str) -> Result<Vec<u8>>;
    async fn list(&self, actor: &str, company: &str, search: Search) -> Result<DraftPage>;
    async fn get(&self, actor: &str, company: &str, id: &str) -> Result<Draft>;
    async fn create(&self, actor: &str, company: &str, command: Create) -> Result<Draft>;
    async fn update(&self, actor: &str, company: &str, id: &str, command: Update) -> Result<Draft>;
}
pub async fn list(
    store: &dyn DraftStore,
    actor: &str,
    company: &str,
    search: Search,
) -> Result<DraftPage> {
    let search = validate_search(search)?;
    store.list(actor, company, search).await
}
pub async fn get(store: &dyn DraftStore, actor: &str, company: &str, id: &str) -> Result<Draft> {
    store.get(actor, company, id).await
}
pub async fn create(
    store: &dyn DraftStore,
    actor: &str,
    company: &str,
    command: Create,
) -> Result<Draft> {
    let mut errors = faktura_domain::invoice::validate_draft(&command.data);
    if uuid::Uuid::parse_str(&command.request_id).is_err() {
        errors.push("request_id".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    store.create(actor, company, command).await
}
pub async fn update(
    store: &dyn DraftStore,
    actor: &str,
    company: &str,
    id: &str,
    command: Update,
) -> Result<Draft> {
    let mut errors = faktura_domain::invoice::validate_draft(&command.data);
    if command.revision < 1 {
        errors.push("revision".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    store.update(actor, company, id, command).await
}

pub async fn issue(
    store: &dyn DraftStore,
    actor: &str,
    company: &str,
    id: &str,
    command: Issue,
) -> Result<Draft> {
    if uuid::Uuid::parse_str(&command.request_id).is_err() || command.revision < 1 {
        return Err(Error::Invalid(vec!["issue_request".into()]));
    }
    store.issue(actor, company, id, command).await
}
pub async fn pdf(store: &dyn DraftStore, actor: &str, company: &str, id: &str) -> Result<Vec<u8>> {
    store.pdf(actor, company, id).await
}

pub fn validate_search(mut search: Search) -> Result<Search> {
    search.query = search.query.trim().to_owned();
    let invalid_date = |v: &Option<String>| {
        v.as_ref().is_some_and(|v| {
            v.len() != 10 || chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err()
        })
    };
    let amount = |v: &Option<String>| -> Result<Option<rust_decimal::Decimal>> {
        v.as_ref()
            .map(|v| {
                let parsed = v
                    .parse::<rust_decimal::Decimal>()
                    .map_err(|_| Error::Invalid(vec!["search".into()]))?;
                if v.len() > 20 || parsed.is_sign_negative() || parsed.scale() > 2 {
                    return Err(Error::Invalid(vec!["search".into()]));
                }
                Ok(parsed)
            })
            .transpose()
    };
    let min = amount(&search.amount_min)?;
    let max = amount(&search.amount_max)?;
    if invalid_date(&search.date_from)
        || invalid_date(&search.date_to)
        || !["issued_on", "due_on"].contains(&search.date_field.as_str())
        || matches!((&search.date_from, &search.date_to), (Some(a), Some(b)) if a > b)
        || matches!((min, max), (Some(a), Some(b)) if a > b)
    {
        return Err(Error::Invalid(vec!["search".into()]));
    }

    if ![
        "all",
        "draft",
        "issued",
        "outstanding",
        "overdue",
        "paid",
        "partial",
        "overpaid",
    ]
    .contains(&search.status.as_str())
        || search.query.chars().count() > 200
        || !(1..=100).contains(&search.page_size)
        || !(1..=10000).contains(&search.page)
        || ![
            "updated_desc",
            "issued_desc",
            "issued_asc",
            "customer_asc",
            "customer_desc",
            "number_desc",
            "number_asc",
            "due_asc",
            "due_desc",
            "amount_asc",
            "amount_desc",
        ]
        .contains(&search.sort.as_str())
        || search
            .customer_id
            .as_ref()
            .is_some_and(|id| id.parse::<i64>().map_or(true, |v| v <= 0))
    {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    Ok(search)
}

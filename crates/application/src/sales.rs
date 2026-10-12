//! Shared template/quotation commands, used by REST and MCP.
pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use faktura_domain::invoice::{DraftData, Totals};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Template,
    Quote,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub name: String,
    pub maturity_days: i64,
    pub status: String,
    pub invoice: DraftData,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub company_id: String,
    pub kind: Kind,
    pub revision: i64,
    pub archived: bool,
    pub number: Option<String>,
    pub invoice_id: Option<String>,
    pub data: Data,
    pub totals: Option<Totals>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Document>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub kind: Kind,
    pub data: Data,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub revision: i64,
    pub archived: bool,
    pub data: Data,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Convert {
    pub request_id: String,
    pub revision: i64,
    pub issued_on: chrono::NaiveDate,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub kind: Kind,
    pub query: String,
    pub status: String,
    pub archived: bool,
    pub page: i64,
    pub page_size: i64,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            kind: Kind::Quote,
            query: String::new(),
            status: "all".into(),
            archived: false,
            page: 1,
            page_size: 25,
        }
    }
}
#[async_trait]
pub trait SalesStore: Send + Sync {
    async fn list(&self, a: &str, c: &str, s: Search) -> Result<Page>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Document>;
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Document>;
    async fn update(&self, a: &str, c: &str, id: &str, cmd: Update) -> Result<Document>;
    async fn convert(
        &self,
        a: &str,
        c: &str,
        id: &str,
        cmd: Convert,
    ) -> Result<crate::draft::Draft>;
}
pub fn validate(data: &Data, kind: &Kind) -> Result<()> {
    let mut errors = faktura_domain::invoice::validate_draft(&data.invoice);
    if data.name.trim().is_empty()
        || data.name.len() > 200
        || data.name.chars().any(char::is_control)
    {
        errors.push("name".into());
    }
    if !(0..=365).contains(&data.maturity_days) {
        errors.push("maturity_days".into());
    }
    if !["created", "sent", "ordered", "invoiced", "rejected"].contains(&data.status.as_str()) {
        errors.push("status".into());
    }
    for date in [
        data.invoice.issued_on,
        data.invoice.due_on,
        data.invoice.tax_on,
    ]
    .into_iter()
    .flatten()
    {
        if !("1900-01-01"..="9999-12-31").contains(&date.to_string().as_str()) {
            errors.push("issued_on".into());
        }
    }
    if *kind == Kind::Quote {
        if data
            .invoice
            .customer
            .as_ref()
            .is_none_or(|c| c.name.trim().is_empty())
        {
            errors.push("customer.name".into());
        }
        if data.invoice.issued_on.is_none()
            || data.invoice.due_on.is_none()
            || data.invoice.due_on < data.invoice.issued_on
        {
            errors.push("due_on".into());
        }
        if let Err(e) = faktura_domain::invoice::calculate(&data.invoice.calculation) {
            errors.extend(e);
        }
        if data.invoice.calculation.lines.is_empty()
            || data
                .invoice
                .calculation
                .lines
                .iter()
                .any(|l| l.description.trim().is_empty())
        {
            errors.push("lines".into());
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(errors))
    }
}
fn uuid(v: &str) -> Result<()> {
    uuid::Uuid::parse_str(v)
        .map(|_| ())
        .map_err(|_| Error::Invalid(vec!["request_id".into()]))
}
pub async fn list(s: &dyn SalesStore, a: &str, c: &str, mut q: Search) -> Result<Page> {
    q.query = q.query.trim().into();
    if q.query.len() > 200
        || !(1..=10000).contains(&q.page)
        || !(1..=100).contains(&q.page_size)
        || !["all", "created", "sent", "ordered", "invoiced", "rejected"]
            .contains(&q.status.as_str())
    {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    s.list(a, c, q).await
}
pub async fn get(s: &dyn SalesStore, a: &str, c: &str, id: &str) -> Result<Document> {
    s.get(a, c, id).await
}
pub async fn create(s: &dyn SalesStore, a: &str, c: &str, mut cmd: Create) -> Result<Document> {
    uuid(&cmd.request_id)?;
    cmd.data.name = cmd.data.name.trim().into();
    validate(&cmd.data, &cmd.kind)?;
    s.create(a, c, cmd).await
}
pub async fn update(
    s: &dyn SalesStore,
    a: &str,
    c: &str,
    id: &str,
    mut cmd: Update,
) -> Result<Document> {
    if cmd.revision < 1 {
        return Err(Error::Invalid(vec!["revision".into()]));
    }
    cmd.data.name = cmd.data.name.trim().into();
    let old = s.get(a, c, id).await?;
    validate(&cmd.data, &old.kind)?;
    s.update(a, c, id, cmd).await
}
pub async fn convert(
    s: &dyn SalesStore,
    a: &str,
    c: &str,
    id: &str,
    cmd: Convert,
) -> Result<crate::draft::Draft> {
    uuid(&cmd.request_id)?;
    if cmd.revision < 1
        || !("1900-01-01"..="9999-12-31").contains(&cmd.issued_on.to_string().as_str())
    {
        return Err(Error::Invalid(vec!["revision".into(), "issued_on".into()]));
    }
    s.convert(a, c, id, cmd).await
}

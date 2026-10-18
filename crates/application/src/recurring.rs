pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use faktura_domain::{invoice::DraftData, recurring::Schedule};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mail {
    pub recipient: String,
    pub subject: String,
    pub body: String,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatePolicy {
    #[default]
    Stored,
    CnbIssueDate,
}
impl RatePolicy {
    fn is_stored(&self) -> bool {
        *self == Self::Stored
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Data {
    #[serde(default, skip_serializing_if = "RatePolicy::is_stored")]
    pub rate_policy: RatePolicy,
    pub name: String,
    pub schedule: Schedule,
    pub maturity_days: u32,
    pub auto_issue: bool,
    pub mail: Option<Mail>,
    pub invoice: DraftData,
}
#[derive(Serialize, Deserialize)]
pub struct Occurrence {
    pub index: i32,
    pub scheduled_on: String,
    pub invoice_id: String,
    pub number: Option<String>,
    pub delivery_status: Option<String>,
    pub error_code: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub company_id: String,
    pub revision: i64,
    pub status: String,
    pub generated: i32,
    pub error_code: Option<String>,
    pub data: Data,
    pub next_dates: Vec<chrono::NaiveDate>,
    pub occurrences: Vec<Occurrence>,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Rule>,
    pub total: i64,
    pub page: i64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub data: Data,
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
    Save { data: Box<Data> },
    Activate,
    Pause,
    Run,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub query: String,
    pub page: Option<i64>,
}
#[async_trait]
pub trait RecurringStore: Send + Sync {
    async fn preview(&self, a: &str, c: &str, cmd: Preview) -> Result<Vec<chrono::NaiveDate>>;
    async fn create(&self, a: &str, c: &str, cmd: Create) -> Result<Rule>;
    async fn command(&self, a: &str, c: &str, id: &str, cmd: Command) -> Result<Rule>;
    async fn get(&self, a: &str, c: &str, id: &str) -> Result<Rule>;
    async fn list(&self, a: &str, c: &str, q: Search) -> Result<Page>;
}
pub fn validate(d: &Data) -> Result<()> {
    let mut errors = faktura_domain::invoice::validate_draft(&d.invoice);
    if d.rate_policy == RatePolicy::CnbIssueDate && d.invoice.calculation.currency == "CZK" {
        errors.push("rate_policy".into());
    }
    if d.name.trim().is_empty()
        || d.name.chars().count() > 200
        || d.name.chars().any(char::is_control)
    {
        errors.push("name".into());
    }
    if !d.schedule.valid() || d.maturity_days > 365 || d.schedule.date(0).is_none() {
        errors.push("schedule".into());
    }
    if let Some(m) = &d.mail {
        if !d.auto_issue
            || m.recipient.is_empty()
            || m.recipient.len() > 254
            || m.recipient.chars().any(char::is_control)
            || m.subject.trim().is_empty()
            || m.subject.chars().count() > 200
            || m.subject.chars().any(char::is_control)
            || m.body.trim().is_empty()
            || m.body.len() > 20000
            || m.body
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            errors.push("mail".into());
        }
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    Ok(())
}
pub fn materialize(d: &Data, on: chrono::NaiveDate) -> Result<DraftData> {
    let mut v = d.invoice.clone();
    v.issued_on = Some(on);
    v.due_on = on.checked_add_days(chrono::Days::new(d.maturity_days.into()));
    v.tax_on = (v.calculation.tax_mode == faktura_domain::invoice::TaxMode::VatPayer).then_some(on);
    faktura_domain::invoice::validate_issue(&v).map_err(Error::Invalid)?;
    Ok(v)
}
pub async fn create(s: &dyn RecurringStore, a: &str, c: &str, cmd: Create) -> Result<Rule> {
    validate(&cmd.data)?;
    if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
        return Err(Error::Invalid(vec!["request".into()]));
    }
    s.create(a, c, cmd).await
}
pub async fn command(
    s: &dyn RecurringStore,
    a: &str,
    c: &str,
    id: &str,
    cmd: Command,
) -> Result<Rule> {
    if let Action::Save { data } = &cmd.action {
        validate(data)?;
    }
    if cmd.revision < 1 || uuid::Uuid::parse_str(&cmd.request_id).is_err() {
        return Err(Error::Invalid(vec!["request".into()]));
    }
    s.command(a, c, id, cmd).await
}
pub async fn list(s: &dyn RecurringStore, a: &str, c: &str, q: Search) -> Result<Page> {
    if q.query.len() > 200 || !(1..=10000).contains(&q.page.unwrap_or(1)) {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    s.list(a, c, q).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub schedule: Schedule,
    pub index: u32,
}

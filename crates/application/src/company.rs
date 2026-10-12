use async_trait::async_trait;
use faktura_domain::company::CompanySettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxProfile {
    pub mode: String,
    pub effective_from: String,
    pub revision: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Company {
    pub id: String,
    pub revision: i64,
    pub step: i32,
    pub completed: bool,
    pub role: String,
    pub settings: CompanySettings,
    pub tax_history: Vec<TaxProfile>,
    pub missing_fields: Vec<String>,
    pub number_preview: String,
    pub can_issue: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CompanyError {
    Invalid(Vec<String>),
    NotFound,
    Forbidden,
    Conflict,
    Unavailable,
}
pub type Result<T> = std::result::Result<T, CompanyError>;
#[async_trait]
pub trait CompanyStore: Send + Sync {
    async fn delete(&self, actor: &str, id: &str, revision: i64) -> Result<()>;
    async fn list(&self, actor: &str) -> Result<Vec<Company>>;
    async fn get(&self, actor: &str, id: &str) -> Result<Company>;
    async fn create(&self, actor: &str, key: &str, settings: CompanySettings) -> Result<Company>;
    async fn update(
        &self,
        actor: &str,
        id: &str,
        revision: i64,
        step: i32,
        complete: bool,
        settings: CompanySettings,
    ) -> Result<Company>;
}
pub async fn list(store: &dyn CompanyStore, actor: &str) -> Result<Vec<Company>> {
    store.list(actor).await
}
pub async fn context(store: &dyn CompanyStore, actor: &str, id: &str) -> Result<Company> {
    if id.parse::<i64>().is_err() {
        return Err(CompanyError::NotFound);
    }
    store.get(actor, id).await
}
pub async fn create(
    store: &dyn CompanyStore,
    actor: &str,
    key: &str,
    mut settings: CompanySettings,
) -> Result<Company> {
    if uuid::Uuid::parse_str(key).is_err() {
        return Err(CompanyError::Invalid(vec!["request_id".into()]));
    }
    settings.normalize();
    let issues = settings.issues(false);
    if !issues.is_empty() {
        return Err(CompanyError::Invalid(issues));
    }
    store.create(actor, key, settings).await
}
pub async fn update(
    store: &dyn CompanyStore,
    actor: &str,
    id: &str,
    revision: i64,
    step: i32,
    complete: bool,
    mut settings: CompanySettings,
) -> Result<Company> {
    // Authorize before exposing validation details, then re-check under the write lock.
    let current = context(store, actor, id).await?;
    if current.role != "owner" {
        return Err(CompanyError::Forbidden);
    }
    if revision < 1 || !(0..=3).contains(&step) {
        return Err(CompanyError::Invalid(vec![
            "revision".into(),
            "step".into(),
        ]));
    }
    settings.normalize();
    let issues = settings.issues(complete || current.completed);
    if !issues.is_empty() {
        return Err(CompanyError::Invalid(issues));
    }
    store
        .update(actor, id, revision, step, complete, settings)
        .await
}

pub async fn delete(store: &dyn CompanyStore, actor: &str, id: &str, revision: i64) -> Result<()> {
    if revision < 1 {
        return Err(CompanyError::Invalid(vec!["revision".into()]));
    }
    store.delete(actor, id, revision).await
}

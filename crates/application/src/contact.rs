pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use faktura_domain::contact::ContactData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub company_id: String,
    pub revision: i64,
    pub archived: bool,
    pub data: ContactData,
}
#[derive(Debug, Serialize)]
pub struct ContactPage {
    pub items: Vec<Contact>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    pub query: String,
    pub archived: bool,
    pub page: i64,
    pub page_size: i64,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            query: String::new(),
            archived: false,
            page: 1,
            page_size: 25,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub request_id: String,
    pub data: ContactData,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub revision: i64,
    pub archived: bool,
    pub data: ContactData,
}
#[async_trait]
pub trait ContactStore: Send + Sync {
    async fn list(&self, actor: &str, company: &str, search: Search) -> Result<ContactPage>;
    async fn get(&self, actor: &str, company: &str, id: &str) -> Result<Contact>;
    async fn create(&self, actor: &str, company: &str, command: Create) -> Result<Contact>;
    async fn update(
        &self,
        actor: &str,
        company: &str,
        id: &str,
        command: Update,
    ) -> Result<Contact>;
}
pub async fn list(
    store: &dyn ContactStore,
    actor: &str,
    company: &str,
    mut search: Search,
) -> Result<ContactPage> {
    search.query = search.query.trim().to_owned();
    if search.query.chars().count() > 200
        || !(1..=100).contains(&search.page_size)
        || !(1..=10000).contains(&search.page)
    {
        return Err(Error::Invalid(vec!["search".into()]));
    }
    store.list(actor, company, search).await
}
pub async fn get(
    store: &dyn ContactStore,
    actor: &str,
    company: &str,
    id: &str,
) -> Result<Contact> {
    store.get(actor, company, id).await
}
pub async fn create(
    store: &dyn ContactStore,
    actor: &str,
    company: &str,
    mut command: Create,
) -> Result<Contact> {
    let mut errors = command.data.normalize_validate();
    if uuid::Uuid::parse_str(&command.request_id).is_err() {
        errors.push("request_id".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    store.create(actor, company, command).await
}
pub async fn update(
    store: &dyn ContactStore,
    actor: &str,
    company: &str,
    id: &str,
    mut command: Update,
) -> Result<Contact> {
    let mut errors = command.data.normalize_validate();
    if command.revision < 1 {
        errors.push("revision".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    store.update(actor, company, id, command).await
}

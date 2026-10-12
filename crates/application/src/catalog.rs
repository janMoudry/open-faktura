pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use faktura_domain::catalog::CatalogData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub id: String,
    pub company_id: String,
    pub revision: i64,
    pub archived: bool,
    pub data: CatalogData,
}
#[derive(Debug, Serialize)]
pub struct CatalogPage {
    pub items: Vec<Catalog>,
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
    pub data: CatalogData,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub revision: i64,
    pub archived: bool,
    pub data: CatalogData,
}
#[async_trait]
pub trait CatalogStore: Send + Sync {
    async fn list(&self, actor: &str, company: &str, search: Search) -> Result<CatalogPage>;
    async fn get(&self, actor: &str, company: &str, id: &str) -> Result<Catalog>;
    async fn create(&self, actor: &str, company: &str, command: Create) -> Result<Catalog>;
    async fn update(
        &self,
        actor: &str,
        company: &str,
        id: &str,
        command: Update,
    ) -> Result<Catalog>;
}
pub async fn list(
    store: &dyn CatalogStore,
    actor: &str,
    company: &str,
    mut search: Search,
) -> Result<CatalogPage> {
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
    store: &dyn CatalogStore,
    actor: &str,
    company: &str,
    id: &str,
) -> Result<Catalog> {
    store.get(actor, company, id).await
}
pub async fn create(
    store: &dyn CatalogStore,
    actor: &str,
    company: &str,
    mut command: Create,
) -> Result<Catalog> {
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
    store: &dyn CatalogStore,
    actor: &str,
    company: &str,
    id: &str,
    mut command: Update,
) -> Result<Catalog> {
    let mut errors = command.data.normalize_validate();
    if command.revision < 1 {
        errors.push("revision".into());
    }
    if !errors.is_empty() {
        return Err(Error::Invalid(errors));
    }
    store.update(actor, company, id, command).await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineOptions {
    pub price_mode: faktura_domain::invoice::PriceMode,
    pub tax_mode: faktura_domain::invoice::TaxMode,
}
pub async fn line(
    store: &dyn CatalogStore,
    actor: &str,
    company: &str,
    id: &str,
    options: LineOptions,
) -> Result<faktura_domain::invoice::Line> {
    let item = store.get(actor, company, id).await?;
    if item.archived {
        return Err(Error::Invalid(vec!["archived".into()]));
    }
    item.data
        .invoice_line(
            uuid::Uuid::new_v4().to_string(),
            options.price_mode,
            options.tax_mode,
        )
        .map_err(Error::Invalid)
}

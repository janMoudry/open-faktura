//! EXPORT-01: read-only, bounded filtered register export.
use crate::{
    company::{CompanyError as Error, Result},
    contact, draft,
};
use async_trait::async_trait;
use serde::Deserialize;
#[derive(Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Xlsx,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Cs,
    En,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Selection {
    Invoices { search: draft::Search },
    Contacts { search: contact::Search },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub format: Format,
    pub language: Language,
    pub columns: Vec<String>,
    pub selection: Selection,
}
pub struct File {
    pub filename: String,
    pub media_type: &'static str,
    pub bytes: Vec<u8>,
}
pub const INVOICE_COLUMNS: &[&str] = &[
    "number",
    "description",
    "status",
    "customer",
    "registration_number",
    "issued_on",
    "due_on",
    "currency",
    "net",
    "vat",
    "payable",
    "paid",
    "outstanding",
    "variable_symbol",
];
pub const CONTACT_COLUMNS: &[&str] = &[
    "name",
    "registration_number",
    "vat_number",
    "email",
    "phone",
    "street",
    "city",
    "postal_code",
    "country",
    "bank_account",
    "iban",
    "archived",
];
#[async_trait]
pub trait ExportStore: Send + Sync {
    async fn export(&self, actor: &str, company: &str, request: Request) -> Result<File>;
}
pub async fn run(
    store: &dyn ExportStore,
    actor: &str,
    company: &str,
    mut request: Request,
) -> Result<File> {
    let allowed = match &mut request.selection {
        Selection::Invoices { search } => {
            *search = draft::validate_search(search.clone())?;
            INVOICE_COLUMNS
        }
        Selection::Contacts { search } => {
            *search = contact::validate_search(search.clone())?;
            CONTACT_COLUMNS
        }
    };
    if request.columns.is_empty()
        || request.columns.len() > allowed.len()
        || request
            .columns
            .iter()
            .enumerate()
            .any(|(i, c)| !allowed.contains(&c.as_str()) || request.columns[..i].contains(c))
    {
        return Err(Error::Invalid(vec!["columns".into()]));
    }
    store.export(actor, company, request).await
}

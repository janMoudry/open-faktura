//! Explicit directory-only anonymization. Financial snapshots are preserved.
pub use crate::company::{CompanyError as Error, Result};
use crate::contact::Contact;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub request_id: String,
    pub revision: i64,
    pub confirm: bool,
}
#[async_trait]
pub trait AnonymizationStore: Send + Sync {
    async fn execute(
        &self,
        actor: &str,
        company: &str,
        contact: &str,
        command: Command,
    ) -> Result<Contact>;
}

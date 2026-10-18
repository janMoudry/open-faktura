pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Invoice,
    Sales,
    Received,
    Contact,
}
impl Kind {
    pub fn table(&self) -> &'static str {
        match self {
            Self::Invoice => "invoice_draft",
            Self::Sales => "sales_document",
            Self::Received => "received_invoice",
            Self::Contact => "contact",
        }
    }
    pub fn column(&self) -> &'static str {
        match self {
            Self::Invoice => "invoice_id",
            Self::Sales => "sales_id",
            Self::Received => "received_id",
            Self::Contact => "contact_id",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub kind: Kind,
    pub id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Create {
        name: String,
    },
    Rename {
        tag_id: String,
        revision: i64,
        name: String,
    },
    Delete {
        tag_id: String,
        revision: i64,
    },
    Attach {
        tag_id: String,
        target: Target,
    },
    Detach {
        tag_id: String,
        target: Target,
    },
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Command {
    pub request_id: String,
    #[serde(flatten)]
    pub action: Action,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub revision: i64,
    pub name: String,
    pub usage_count: i64,
}
#[async_trait]
pub trait TagStore: Send + Sync {
    async fn list(&self, actor: &str, company: &str, target: Option<Target>) -> Result<Vec<Tag>>;
    async fn execute(&self, actor: &str, company: &str, command: Command) -> Result<Vec<Tag>>;
}

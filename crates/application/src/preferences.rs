pub use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    System,
    Light,
    Dark,
}
impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub revision: i64,
    pub theme: Theme,
}
#[async_trait]
pub trait PreferenceStore: Send + Sync {
    async fn get(&self, actor: &str) -> Result<Preferences>;
    async fn set(&self, actor: &str, input: Preferences) -> Result<Preferences>;
}

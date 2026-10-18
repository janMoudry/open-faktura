use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Finance,
    Tax,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Detailed,
    Compact,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    pub kind: Kind,
    pub variant: Variant,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub revision: i64,
    pub widgets: Vec<Widget>,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            revision: 0,
            widgets: vec![
                Widget {
                    kind: Kind::Finance,
                    variant: Variant::Detailed,
                },
                Widget {
                    kind: Kind::Tax,
                    variant: Variant::Detailed,
                },
            ],
        }
    }
}
pub fn validate(input: &Layout) -> Result<()> {
    if !(0..=1_000_000_000_000).contains(&input.revision)
        || input.widgets.len() > 2
        || input
            .widgets
            .iter()
            .enumerate()
            .any(|(i, w)| input.widgets[..i].iter().any(|v| v.kind == w.kind))
    {
        return Err(Error::Invalid(vec!["widgets".into()]));
    }
    Ok(())
}
#[async_trait]
pub trait DashboardWidgetStore: Send + Sync {
    async fn get(&self, actor: &str) -> Result<Layout>;
    async fn set(&self, actor: &str, input: Layout) -> Result<Layout>;
}

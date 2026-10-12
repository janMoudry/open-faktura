use crate::company::{CompanyError as Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub revision: i64,
    pub mode: String,
    pub host: String,
    pub port: u16,
    pub security: String,
    pub sender: String,
    pub username: String,
}
#[derive(Serialize)]
pub struct State {
    #[serde(flatten)]
    pub settings: Settings,
    pub password_set: bool,
    pub encryption_ready: bool,
    pub configured: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    #[serde(flatten)]
    pub settings: Settings,
    pub password: Option<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Test {
    pub request_id: String,
    pub revision: i64,
    pub recipient: String,
    pub language: String,
}
#[derive(Serialize, Deserialize)]
pub struct TestResult {
    pub request_id: String,
    pub status: String,
    pub error_code: Option<String>,
}
#[async_trait]
pub trait SmtpStore: Send + Sync {
    async fn get(&self, actor: &str) -> Result<State>;
    async fn save(&self, actor: &str, cmd: Save) -> Result<State>;
    async fn test(&self, actor: &str, cmd: Test) -> Result<TestResult>;
}
pub async fn save(store: &dyn SmtpStore, actor: &str, mut cmd: Save) -> Result<State> {
    let s = &mut cmd.settings;
    s.host = s.host.trim().to_owned();
    s.sender = s.sender.trim().to_owned();
    s.username = s.username.trim().to_owned();
    if s.revision < 0
        || !["environment", "custom", "disabled"].contains(&s.mode.as_str())
        || !["starttls", "tls", "plain"].contains(&s.security.as_str())
        || s.port == 0
        || s.host.len() > 253
        || s.host
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || ['/', '\\', '@'].contains(&c))
        || s.sender.len() > 254
        || s.username.len() > 254
        || s.username.chars().any(char::is_control)
        || cmd
            .password
            .as_ref()
            .is_some_and(|p| p.len() > 4096 || p.contains(['\r', '\n', '\0']))
    {
        return Err(Error::Invalid(vec!["settings".into()]));
    }
    store.save(actor, cmd).await
}
pub async fn test(store: &dyn SmtpStore, actor: &str, cmd: Test) -> Result<TestResult> {
    if uuid::Uuid::parse_str(&cmd.request_id).is_err()
        || cmd.revision < 0
        || !["cs", "en"].contains(&cmd.language.as_str())
        || cmd.recipient.len() > 254
        || cmd.recipient.chars().any(char::is_control)
    {
        return Err(Error::Invalid(vec!["test".into()]));
    }
    store.test(actor, cmd).await
}

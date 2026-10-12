use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub id: String,
    pub email: String,
    pub name: String,
    pub installation_admin: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    InvalidInput,
    EmailTaken,
    InvalidPassword,
    Conflict,
    Unauthorized,
    AlreadyConfigured,
    RateLimited,
    Unavailable,
}
pub type AuthResult<T> = Result<T, AuthError>;

// Port implementations own hashing and atomic persistence. HTTP/MCP share these
// commands; no session or installation state is inferred from a client's route.
#[async_trait]
pub trait AuthStore: Send + Sync {
    async fn setup_required(&self) -> AuthResult<bool>;
    async fn bootstrap(&self, secret: &str, email: &str, password: &str) -> AuthResult<()>;
    async fn login(&self, email: &str, password: &str) -> AuthResult<(Identity, String)>;
    async fn session(&self, token: &str) -> AuthResult<Identity>;
    async fn logout(&self, token: &str) -> AuthResult<()>;
    async fn update_account(
        &self,
        token: &str,
        name: &str,
        email: &str,
        current_password: &str,
        password: Option<&str>,
    ) -> AuthResult<Identity>;
    async fn recover(&self, email: &str, password: &str) -> AuthResult<()>;
}

pub fn email(value: &str) -> AuthResult<String> {
    let normalized = value.trim().to_lowercase();
    let parts: Vec<_> = normalized.split('@').collect();
    if normalized.len() > 254
        || parts.len() != 2
        || parts[0].is_empty()
        || !parts[1].contains('.')
        || parts[1].starts_with('.')
        || parts[1].ends_with('.')
        || normalized.chars().any(char::is_whitespace)
        || normalized.chars().any(char::is_control)
    {
        return Err(AuthError::InvalidInput);
    }
    Ok(normalized)
}
pub fn new_password(value: &str) -> AuthResult<()> {
    if value.chars().count() < 12 || value.len() > 1024 {
        return Err(AuthError::InvalidInput);
    }
    Ok(())
}
pub async fn bootstrap(
    store: &dyn AuthStore,
    secret: &str,
    address: &str,
    password: &str,
) -> AuthResult<()> {
    new_password(password)?;
    store.bootstrap(secret, &email(address)?, password).await
}
pub async fn login(
    store: &dyn AuthStore,
    address: &str,
    password: &str,
) -> AuthResult<(Identity, String)> {
    if password.is_empty() || password.len() > 1024 {
        return Err(AuthError::Unauthorized);
    }
    let address = email(address).map_err(|_| AuthError::Unauthorized)?;
    store.login(&address, password).await
}
pub async fn recover(store: &dyn AuthStore, address: &str, password: &str) -> AuthResult<()> {
    new_password(password)?;
    store.recover(&email(address)?, password).await
}

pub async fn update_account(
    store: &dyn AuthStore,
    token: &str,
    name: &str,
    address: &str,
    current_password: &str,
    password: Option<&str>,
) -> AuthResult<Identity> {
    let name = name.trim();
    if name.chars().count() > 120
        || name.chars().any(char::is_control)
        || current_password.len() > 1024
    {
        return Err(AuthError::InvalidInput);
    }
    if let Some(value) = password {
        new_password(value)?;
    }
    store
        .update_account(token, name, &email(address)?, current_password, password)
        .await
}

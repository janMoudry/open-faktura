use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_application::auth::{self, AuthError, AuthStore, Identity};
use faktura_infrastructure::auth::PgAuth;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthState {
    pub store: Arc<dyn AuthStore>,
    pub origins: Vec<String>,
    pub secure: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bootstrap {
    pub secret: String,
    pub email: String,
    pub password: String,
}
#[derive(Serialize)]
pub struct User {
    id: String,
    email: String,
    name: String,
    installation_admin: bool,
}
impl From<Identity> for User {
    fn from(v: Identity) -> Self {
        Self {
            id: v.id,
            email: v.email,
            name: v.name,
            installation_admin: v.installation_admin,
        }
    }
}
#[derive(Serialize)]
pub struct Setup {
    setup_required: bool,
}

pub fn router(pool: PgPool, secret: Option<String>, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/auth/setup", get(setup).post(bootstrap))
        .route("/api/auth/login", post(login))
        .route("/api/auth/session", get(session))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/account", post(update_account))
        .layer(DefaultBodyLimit::max(8192))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
        .with_state(AuthState {
            store: Arc::new(PgAuth::new(pool, secret)),
            origins,
            secure,
        })
}
pub(crate) struct Failure(pub(crate) AuthError);
impl From<AuthError> for Failure {
    fn from(e: AuthError) -> Self {
        Self(e)
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (status, code) = match self.0 {
            AuthError::EmailTaken => (StatusCode::CONFLICT, "email_taken"),
            AuthError::Conflict => (StatusCode::CONFLICT, "conflict"),
            AuthError::InvalidPassword => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_password"),
            AuthError::InvalidInput => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_input"),
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AuthError::AlreadyConfigured => (StatusCode::CONFLICT, "already_configured"),
            AuthError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            AuthError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        };
        let mut response = (status, Json(serde_json::json!({"code":code}))).into_response();
        if status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("900"));
        }
        response
    }
}
pub(crate) fn same_origin(state: &AuthState, headers: &HeaderMap) -> Result<(), Failure> {
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    if origin.is_none()
        || !state
            .origins
            .iter()
            .any(|allowed| Some(allowed.as_str()) == origin)
        || headers
            .get("x-faktura-request")
            .and_then(|v| v.to_str().ok())
            != Some("1")
    {
        return Err(Failure(AuthError::Unauthorized));
    }
    Ok(())
}
fn cookie_name(secure: bool) -> &'static str {
    if secure {
        "__Host-faktura_session"
    } else {
        "faktura_session"
    }
}
pub(crate) fn token(headers: &HeaderMap, secure: bool) -> String {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';').find_map(|part| {
                let (name, value) = part.trim().split_once('=')?;
                (name == cookie_name(secure)).then_some(value.to_owned())
            })
        })
        .unwrap_or_default()
}
fn session_cookie(value: &str, secure: bool, age: u32) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={age}{}",
        cookie_name(secure),
        if secure { "; Secure" } else { "" }
    ))
    .expect("generated hex cookie")
}
async fn setup(State(state): State<AuthState>) -> Result<Json<Setup>, Failure> {
    Ok(Json(Setup {
        setup_required: state.store.setup_required().await?,
    }))
}
async fn bootstrap(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(body): Json<Bootstrap>,
) -> Result<StatusCode, Failure> {
    same_origin(&state, &headers)?;
    auth::bootstrap(
        state.store.as_ref(),
        &body.secret,
        &body.email,
        &body.password,
    )
    .await?;
    // Explicit login after successful commit: retries never create a second admin.
    Ok(StatusCode::CREATED)
}
async fn login(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(body): Json<Credentials>,
) -> Result<Response, Failure> {
    same_origin(&state, &headers)?;
    let (user, new_token) = auth::login(state.store.as_ref(), &body.email, &body.password).await?;
    // Replace an existing browser session, never accept a client-chosen token.
    if let Err(e) = state.store.logout(&token(&headers, state.secure)).await {
        let _ = state.store.logout(&new_token).await;
        return Err(e.into());
    }
    let mut response = Json(User::from(user)).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(&new_token, state.secure, 43200),
    );
    Ok(response)
}
async fn session(
    State(state): State<AuthState>,
    headers: HeaderMap,
) -> Result<Json<User>, Failure> {
    Ok(Json(
        state
            .store
            .session(&token(&headers, state.secure))
            .await?
            .into(),
    ))
}
async fn logout(State(state): State<AuthState>, headers: HeaderMap) -> Result<Response, Failure> {
    same_origin(&state, &headers)?;
    state.store.logout(&token(&headers, state.secure)).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie("", state.secure, 0));
    Ok(response)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountUpdate {
    name: String,
    email: String,
    #[serde(default)]
    current_password: String,
    new_password: Option<String>,
}
async fn update_account(
    State(state): State<AuthState>,
    headers: HeaderMap,
    Json(body): Json<AccountUpdate>,
) -> Result<Json<User>, Failure> {
    same_origin(&state, &headers)?;
    Ok(Json(
        auth::update_account(
            state.store.as_ref(),
            &token(&headers, state.secure),
            &body.name,
            &body.email,
            &body.current_password,
            body.new_password.as_deref(),
        )
        .await?
        .into(),
    ))
}

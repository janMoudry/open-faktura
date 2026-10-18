use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use faktura_application::contact_anonymization::{AnonymizationStore, Command};
use faktura_infrastructure::{auth::PgAuth, contact_anonymization::PgContactAnonymization};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{company}/contacts/{contact}/anonymize",
            post(write),
        )
        .layer(DefaultBodyLimit::max(4096))
        .with_state(S {
            pool: pool.clone(),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn actor(s: &S, h: &HeaderMap, write: bool) -> Result<String, auth::Failure> {
    if write {
        auth::same_origin(&s.auth, h)?;
    }
    Ok(s.auth
        .store
        .session(&auth::token(h, s.auth.secure))
        .await?
        .id)
}
fn response<T: serde::Serialize>(v: faktura_application::company::Result<T>) -> Response {
    match v {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

async fn write(
    State(s): State<S>,
    h: HeaderMap,
    Path((company, contact)): Path<(String, String)>,
    Json(command): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(
        PgContactAnonymization(s.pool)
            .execute(&a, &company, &contact, command)
            .await,
    )
}
pub async fn tool(
    pool: PgPool,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::company::CompanyError;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    if name != "contact_anonymize" {
        return Err(invalid());
    }
    let contact = args
        .remove("contact_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let command: Command =
        serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?;
    let value = PgContactAnonymization(pool)
        .execute(actor, &company, &contact, command)
        .await?;
    serde_json::to_value(value).map_err(|_| CompanyError::Unavailable)
}

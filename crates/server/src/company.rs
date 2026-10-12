use crate::auth::{self, AuthState};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::company::{self, Company, CompanyError};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct CompanyState {
    store: PgCompany,
    auth: AuthState,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    request_id: String,
    settings: CompanySettings,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    revision: i64,
    step: i32,
    complete: bool,
    settings: CompanySettings,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies", get(list).post(create))
        .route(
            "/api/companies/{id}",
            get(context).put(update).delete(delete),
        )
        .layer(DefaultBodyLimit::max(16384))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                r
            },
        ))
        .with_state(CompanyState {
            store: PgCompany(pool.clone()),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
pub(crate) struct Failure(pub(crate) CompanyError);
impl From<CompanyError> for Failure {
    fn from(e: CompanyError) -> Self {
        Self(e)
    }
}
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let (status, code, fields) = match self.0 {
            CompanyError::Invalid(v) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_input", v),
            CompanyError::NotFound => (StatusCode::NOT_FOUND, "not_found", vec![]),
            CompanyError::Forbidden => (StatusCode::FORBIDDEN, "forbidden", vec![]),
            CompanyError::Conflict => (StatusCode::CONFLICT, "revision_conflict", vec![]),
            CompanyError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable", vec![]),
        };
        (
            status,
            Json(serde_json::json!({"code":code,"fields":fields})),
        )
            .into_response()
    }
}
async fn actor(
    state: &CompanyState,
    h: &HeaderMap,
    write: bool,
) -> std::result::Result<String, auth::Failure> {
    if write {
        auth::same_origin(&state.auth, h)?;
    }
    state
        .auth
        .store
        .session(&auth::token(h, state.auth.secure))
        .await
        .map(|u| u.id)
        .map_err(auth::Failure)
}
async fn list(State(s): State<CompanyState>, h: HeaderMap) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match company::list(&s.store, &a).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn context(State(s): State<CompanyState>, h: HeaderMap, Path(id): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(company::context(&s.store, &a, &id).await)
}
fn response(result: company::Result<Company>) -> Response {
    match result {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn create(
    State(s): State<CompanyState>,
    h: HeaderMap,
    body: std::result::Result<Json<Create>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let b = match body {
        Ok(Json(b)) => b,
        Err(_) => return Failure(CompanyError::Invalid(vec!["request".into()])).into_response(),
    };
    response(company::create(&s.store, &a, &b.request_id, b.settings).await)
}
async fn update(
    State(s): State<CompanyState>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: std::result::Result<Json<Update>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let b = match body {
        Ok(Json(b)) => b,
        Err(_) => return Failure(CompanyError::Invalid(vec!["request".into()])).into_response(),
    };
    response(
        company::update(
            &s.store, &a, &id, b.revision, b.step, b.complete, b.settings,
        )
        .await,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delete {
    revision: i64,
}
async fn delete(
    State(s): State<CompanyState>,
    h: HeaderMap,
    Path(id): Path<String>,
    body: std::result::Result<Json<Delete>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let b = match body {
        Ok(Json(b)) => b,
        Err(_) => return Failure(CompanyError::Invalid(vec!["request".into()])).into_response(),
    };
    match company::delete(&s.store, &a, &id, b.revision).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

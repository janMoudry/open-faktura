use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::document_tags::{Command, TagStore, Target};
use faktura_infrastructure::{auth::PgAuth, document_tags::PgTags};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{company}/tags", get(read).post(write))
        .route(
            "/api/companies/{company}/tags/target",
            axum::routing::post(for_target),
        )
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
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

async fn read(State(s): State<S>, h: HeaderMap, Path(company): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTags(s.pool).list(&a, &company, None).await)
}
async fn for_target(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(target): Json<Target>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTags(s.pool).list(&a, &company, Some(target)).await)
}
async fn write(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(command): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTags(s.pool).execute(&a, &company, command).await)
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
    let store = PgTags(pool);
    let result = if name == "tags_execute" {
        store
            .execute(
                actor,
                &company,
                serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
            )
            .await?
    } else {
        let target = if args.is_empty() {
            None
        } else {
            Some(serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?)
        };
        store.list(actor, &company, target).await?
    };
    serde_json::to_value(result).map_err(|_| CompanyError::Unavailable)
}

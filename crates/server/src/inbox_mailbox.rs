use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_application::inbox_mailbox::{Configure, MailboxStore};
use faktura_infrastructure::{auth::PgAuth, inbox_mailbox::PgMailbox};
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
            "/api/companies/{c}/inbox/mailbox",
            get(settings).post(configure),
        )
        .route("/api/companies/{c}/inbox/mailbox/sync", post(sync))
        .route("/api/companies/{c}/inbox/mailbox/history", get(history))
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
fn response<T: serde::Serialize>(result: faktura_application::company::Result<T>) -> Response {
    match result {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn settings(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgMailbox::new(s.pool).settings(&a, &c).await)
}
async fn configure(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(v): Json<Configure>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgMailbox::new(s.pool).configure(&a, &c, v).await)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Sync {
    request_id: String,
}
async fn sync(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(v): Json<Sync>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgMailbox::new(s.pool).sync(&a, &c, &v.request_id).await)
}
async fn history(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgMailbox::new(s.pool).history(&a, &c).await)
}

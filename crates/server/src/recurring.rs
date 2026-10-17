use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::recurring::{self, Command, Create, RecurringStore, Search};
use faktura_infrastructure::{auth::PgAuth, recurring::PgRecurring};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgRecurring,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/recurring", get(list).post(create))
        .route(
            "/api/companies/{c}/recurring/{id}",
            get(detail).post(command),
        )
        .route(
            "/api/companies/{c}/recurring/preview",
            axum::routing::post(preview),
        )
        .layer(DefaultBodyLimit::max(262144))
        .with_state(S {
            store: PgRecurring(pool.clone()),
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
async fn list(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(recurring::list(&s.store, &a, &c, q).await)
}
async fn detail(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.get(&a, &c, &id).await)
}
async fn create(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<Create>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(recurring::create(&s.store, &a, &c, cmd).await)
}

async fn command(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Json(cmd): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(recurring::command(&s.store, &a, &c, &id, cmd).await)
}

async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<recurring::Preview>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.preview(&a, &c, cmd).await)
}

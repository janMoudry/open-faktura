use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_application::advance_settlement::{self, Configure, Search, SettlementStore};
use faktura_infrastructure::{advance_settlement::PgSettlement, auth::PgAuth};
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgSettlement,
    auth: AuthState,
}
pub fn router(pool: sqlx::PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{c}/advance-settlements",
            get(candidates).post(configure),
        )
        .route(
            "/api/companies/{c}/invoices/{id}/advance-preview",
            post(preview),
        )
        .layer(DefaultBodyLimit::max(262144))
        .with_state(S {
            store: PgSettlement(pool.clone()),
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
async fn configure(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<Configure>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(advance_settlement::configure(&s.store, &a, &c, cmd).await)
}
async fn candidates(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(advance_settlement::candidates(&s.store, &a, &c, q).await)
}
async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Json(calc): Json<faktura_domain::invoice::Calculation>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.preview(&a, &c, &id, calc).await)
}

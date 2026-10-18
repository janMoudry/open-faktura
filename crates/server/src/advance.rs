use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_application::advance::{self, AdvanceStore, Register, Reverse};
use faktura_infrastructure::{advance::PgAdvance, auth::PgAuth};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgAdvance,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/advances/{d}/pdf", get(pdf))
        .route(
            "/api/companies/{c}/advances/{d}/payments",
            get(payments).post(register),
        )
        .route(
            "/api/companies/{c}/advances/{d}/payments/{p}/reverse",
            post(reverse),
        )
        .layer(DefaultBodyLimit::max(16384))
        .with_state(S {
            store: PgAdvance(pool.clone()),
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
async fn pdf(State(s): State<S>, h: HeaderMap, Path((c, d)): Path<(String, String)>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    match s.store.pdf(&a, &c, &d).await {
        Ok(v) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"advance.pdf\"",
                ),
                (header::CACHE_CONTROL, "no-store"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            v,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn payments(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, d)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.payments(&a, &c, &d).await)
}
async fn register(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, d)): Path<(String, String)>,
    Json(cmd): Json<Register>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(advance::register(&s.store, &a, &c, &d, cmd).await)
}
async fn reverse(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, d, p)): Path<(String, String, String)>,
    Json(cmd): Json<Reverse>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(advance::reverse(&s.store, &a, &c, &d, &p, cmd).await)
}

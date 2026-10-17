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
use faktura_application::tax_receipt::{self, Issue, Search, TaxReceiptStore};
use faktura_infrastructure::{auth::PgAuth, tax_receipt::PgTaxReceipt};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgTaxReceipt,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/tax-receipts", get(list).post(issue))
        .route("/api/companies/{c}/tax-receipts/{id}", get(detail))
        .route("/api/companies/{c}/tax-receipts/{id}/pdf", get(pdf))
        .route(
            "/api/companies/{c}/advances/{d}/payments/{p}/tax-preview",
            get(preview),
        )
        .layer(DefaultBodyLimit::max(16384))
        .with_state(S {
            store: PgTaxReceipt(pool.clone()),
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
    response(tax_receipt::list(&s.store, &a, &c, q).await)
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
async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, d, p)): Path<(String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.preview(&a, &c, &d, &p).await)
}
async fn issue(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<Issue>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(tax_receipt::issue(&s.store, &a, &c, cmd).await)
}
async fn pdf(State(s): State<S>, h: HeaderMap, Path((c, id)): Path<(String, String)>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match s.store.pdf(&a, &c, &id).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"tax-receipt.pdf\"",
                ),
                (header::CACHE_CONTROL, "no-store"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

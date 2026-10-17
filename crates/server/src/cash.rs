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
use faktura_application::cash::{CashStore, Command, Search, TaxRequest};
use faktura_infrastructure::{auth::PgAuth, cash::PgCash};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgCash,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/cash/accounts", get(accounts))
        .route("/api/companies/{c}/cash/terminals", get(terminals))
        .route("/api/companies/{c}/cash/settings", get(settings))
        .route("/api/companies/{c}/cash/tax-preview", post(tax_preview))
        .route("/api/companies/{c}/cash/movements/{id}/pdf", get(pdf))
        .route("/api/companies/{c}/cash/balance", get(balance))
        .route("/api/companies/{c}/cash/movements", get(movements))
        .route("/api/companies/{c}/cash/movements/{id}", get(detail))
        .route(
            "/api/companies/{c}/cash/movements/{id}/candidates",
            get(candidates),
        )
        .route("/api/companies/{c}/cash/commands", post(command))
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        .with_state(S {
            store: PgCash(pool.clone()),
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
async fn accounts(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.accounts(&a, &c).await)
}
async fn movements(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.movements(&a, &c, q).await)
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
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Filter {
    query: String,
}
async fn candidates(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Query(q): Query<Filter>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.candidates(&a, &c, &id, &q.query).await)
}
async fn command(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.execute(&a, &c, cmd).await)
}

async fn balance(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.balance(&a, &c, q).await)
}

async fn settings(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.settings(&a, &c).await)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PdfQuery {
    language: String,
}
async fn pdf(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Query(q): Query<PdfQuery>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    match s.store.pdf(&a, &c, &id, &q.language).await {
        Ok(pdf) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (header::CACHE_CONTROL, "no-store"),
                (
                    header::CONTENT_DISPOSITION,
                    "inline; filename=cash-voucher.pdf",
                ),
            ],
            pdf,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

async fn tax_preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(request): Json<TaxRequest>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.tax_preview(&a, &c, request).await)
}
async fn terminals(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.terminals(&a, &c).await)
}

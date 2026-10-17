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
use faktura_application::received::{self, Command, Create, ReceivedStore, Search};
use faktura_infrastructure::{auth::PgAuth, received::PgReceived};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgReceived,
    report: faktura_infrastructure::purchase_report::PgPurchaseReport,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{c}/received-invoices",
            get(list).post(create),
        )
        .route(
            "/api/companies/{c}/received-invoices/{id}",
            get(detail).post(command),
        )
        .route(
            "/api/companies/{c}/received-invoices/preview",
            axum::routing::post(preview),
        )
        .route(
            "/api/companies/{c}/received-invoices/expense-payments",
            get(expense_payments),
        )
        .route(
            "/api/companies/{c}/received-invoices/expense-overview",
            get(expense_overview),
        )
        .layer(DefaultBodyLimit::max(262144))
        .with_state(S {
            store: PgReceived(pool.clone()),
            report: faktura_infrastructure::purchase_report::PgPurchaseReport(pool.clone()),
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
    response(s.store.list(&a, &c, q).await)
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
    response(received::create(&s.store, &a, &c, cmd).await)
}
async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(data): Json<received::Data>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.preview(&a, &c, data).await)
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
    response(received::command(&s.store, &a, &c, &id, cmd).await)
}

async fn expense_payments(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<faktura_application::purchase_report::Search>,
) -> Response {
    use faktura_application::purchase_report::PurchaseReportStore;
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.report.payments(&a, &c, q).await)
}
async fn expense_overview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<faktura_application::purchase_report::Period>,
) -> Response {
    use faktura_application::purchase_report::PurchaseReportStore;
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.report.overview(&a, &c, q).await)
}

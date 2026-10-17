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
use faktura_application::stock::{Command, Search, StockStore};
use faktura_application::stock_report::{CostCommand, Search as ReportSearch, StockReportStore};
use faktura_infrastructure::stock_report::PgStockReport;
use faktura_infrastructure::{auth::PgAuth, stock::PgStock};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/stock/cards", get(cards))
        .route("/api/companies/{c}/stock/report", get(report))
        .route("/api/companies/{c}/stock/costs", post(set_cost))
        .route("/api/companies/{c}/stock/costs/{i}", get(costs))
        .route("/api/companies/{c}/stock/movements", get(movements))
        .route("/api/companies/{c}/stock/commands", post(command))
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

async fn cards(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgStock(s.pool).cards(&a, &c).await)
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
    response(PgStock(s.pool).movements(&a, &c, q).await)
}
async fn command(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgStock(s.pool).execute(&a, &c, d).await)
}

async fn report(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<ReportSearch>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(PgStockReport(s.pool).report(&a, &c, q).await)
}
async fn set_cost(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<CostCommand>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(PgStockReport(s.pool).set_cost(&a, &c, cmd).await)
}
async fn costs(State(s): State<S>, h: HeaderMap, Path((c, i)): Path<(String, String)>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(PgStockReport(s.pool).costs(&a, &c, &i).await)
}

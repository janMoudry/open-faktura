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
use faktura_application::bank_connection::ConnectionStore;
use faktura_application::banking::{
    BalanceQuery, BankStore, BatchInput, Command, FileInput, Search, StatementInput,
};
use faktura_infrastructure::bank_connection::PgConnection;
use faktura_infrastructure::{auth::PgAuth, banking::PgBank};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgBank,
    connection: PgConnection,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{c}/bank/foreign-report",
            get(foreign_report),
        )
        .route("/api/companies/{c}/bank/accounts", get(accounts))
        .route(
            "/api/companies/{c}/bank/movements/{id}/foreign-candidates",
            get(foreign_candidates),
        )
        .route(
            "/api/companies/{c}/bank/connection",
            get(connection_status).post(connection_command),
        )
        .route("/api/companies/{c}/bank/balance", get(balance))
        .route("/api/companies/{c}/bank/batch-preview", post(batch_preview))
        .route(
            "/api/companies/{c}/bank/movements/{id}/suggestions",
            get(suggestions),
        )
        .route(
            "/api/companies/{c}/bank/movements/{id}/split-candidates",
            get(split_candidates),
        )
        .route("/api/companies/{c}/bank/movements", get(movements))
        .route("/api/companies/{c}/bank/movements/{id}", get(detail))
        .route(
            "/api/companies/{c}/bank/movements/{id}/candidates",
            get(candidates),
        )
        .route("/api/companies/{c}/bank/preview", post(preview))
        .route(
            "/api/companies/{c}/bank/statement-preview",
            post(statement_preview),
        )
        .route("/api/companies/{c}/bank/commands", post(command))
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        .with_state(S {
            store: PgBank(pool.clone()),
            connection: PgConnection::new(pool.clone()),
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
async fn split_candidates(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Query(q): Query<Filter>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.split_candidates(&a, &c, &id, &q.query).await)
}
async fn foreign_candidates(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Query(q): Query<Filter>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.foreign_candidates(&a, &c, &id, &q.query).await)
}
async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(f): Json<FileInput>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.preview(&a, &c, f).await)
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

async fn statement_preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(f): Json<StatementInput>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.statement_preview(&a, &c, f).await)
}

async fn suggestions(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.suggestions(&a, &c, &id).await)
}

async fn batch_preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(input): Json<BatchInput>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.batch_preview(&a, &c, input).await)
}

async fn balance(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<BalanceQuery>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.balance(&a, &c, q).await)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionQuery {
    account_id: String,
}
async fn connection_status(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<ConnectionQuery>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.connection.status(&a, &c, &q.account_id).await)
}
async fn connection_command(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(cmd): Json<faktura_application::bank_connection::Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.connection.execute(&a, &c, cmd).await)
}

async fn foreign_report(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<faktura_application::banking::ForeignReportQuery>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.foreign_report(&a, &c, q).await)
}

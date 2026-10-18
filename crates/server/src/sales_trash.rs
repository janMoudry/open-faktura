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
use faktura_application::sales_trash::{Command, TrashStore};
use faktura_infrastructure::{auth::PgAuth, sales_trash::PgSalesTrash};
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
            "/api/companies/{company}/sales-trash",
            get(read).post(write),
        )
        .layer(DefaultBodyLimit::max(8192))
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
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    #[serde(default = "first")]
    page: i64,
}
fn first() -> i64 {
    1
}
async fn read(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(PgSalesTrash(s.pool).list(&a, &c, q.page).await)
}
async fn write(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(v): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(PgSalesTrash(s.pool).execute(&a, &c, v).await)
}
pub async fn tool(
    pool: PgPool,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::company::CompanyError as Error;
    let bad = || Error::Invalid(vec!["arguments".into()]);
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(bad)?;
    let store = PgSalesTrash(pool);
    let v = if name == "sales_trash_list" {
        let q: Search =
            serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| bad())?;
        serde_json::to_value(store.list(actor, &c, q.page).await?)
    } else if name == "sales_trash_execute" {
        let q = serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| bad())?;
        serde_json::to_value(store.execute(actor, &c, q).await?)
    } else {
        return Err(bad());
    };
    v.map_err(|_| Error::Unavailable)
}

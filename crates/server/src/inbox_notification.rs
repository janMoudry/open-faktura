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
use faktura_application::inbox_notification::{Configure, NotificationsStore};
use faktura_infrastructure::{auth::PgAuth, inbox_notification::PgNotifications};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/inbox/notifications", get(feed))
        .route(
            "/api/companies/{c}/inbox/notifications/settings",
            post(configure),
        )
        .route("/api/companies/{c}/inbox/notifications/read", post(read))
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
async fn feed(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgNotifications(s.pool).feed(&a, &c).await)
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
    response(PgNotifications(s.pool).configure(&a, &c, v).await)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    through: String,
}
async fn read(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(v): Json<Read>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgNotifications(s.pool).read(&a, &c, &v.through).await)
}
pub async fn tool(
    pool: PgPool,
    a: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::company::CompanyError as Error;
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| Error::Invalid(vec!["arguments".into()]))?;
    let store = PgNotifications(pool);
    let result = match name {
        "inbox_notification_feed" => {
            if !args.is_empty() {
                return Err(Error::Invalid(vec!["arguments".into()]));
            }
            store.feed(a, &c).await?
        }
        "inbox_notification_configure" => {
            store
                .configure(
                    a,
                    &c,
                    serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| Error::Invalid(vec!["arguments".into()]))?,
                )
                .await?
        }
        "inbox_notification_read" => {
            let v: Read = serde_json::from_value(serde_json::Value::Object(args))
                .map_err(|_| Error::Invalid(vec!["arguments".into()]))?;
            store.read(a, &c, &v.through).await?
        }
        _ => return Err(Error::NotFound),
    };
    serde_json::to_value(result).map_err(|_| Error::Unavailable)
}

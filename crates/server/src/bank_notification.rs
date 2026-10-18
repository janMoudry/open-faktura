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
use faktura_application::bank_notification::{Intake, NotificationStore, Settings};
use faktura_infrastructure::{auth::PgAuth, bank_notification::PgNotification};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/bank/notifications", post(receive))
        .route(
            "/api/companies/{c}/bank/accounts/{account}/notification-settings",
            get(settings).post(configure),
        )
        .route(
            "/api/companies/{c}/bank/accounts/{account}/notification-history",
            get(history),
        )
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
fn response<T: serde::Serialize>(result: faktura_application::company::Result<T>) -> Response {
    match result {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn settings(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, account)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgNotification(s.pool).settings(&a, &c, &account).await)
}
async fn configure(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, account)): Path<(String, String)>,
    Json(v): Json<Settings>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    if v.account_id != account {
        return Failure(faktura_application::company::CompanyError::Invalid(vec![
            "account_id".into(),
        ]))
        .into_response();
    }
    response(PgNotification(s.pool).configure(&a, &c, v).await)
}
async fn history(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, account)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgNotification(s.pool).history(&a, &c, &account).await)
}
async fn receive(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(v): Json<Intake>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgNotification(s.pool).receive(&a, &c, v).await)
}

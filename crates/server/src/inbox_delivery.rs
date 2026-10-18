use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::inbox_delivery::{InboxDeliveryStore, Send};
use faktura_infrastructure::{auth::PgAuth, inbox_delivery::PgInboxDelivery};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    auth: AuthState,
    store: PgInboxDelivery,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool, store: PgInboxDelivery) -> Router {
    Router::new()
        .route(
            "/api/companies/{c}/inbox/{d}/deliveries",
            get(history).post(send),
        )
        .with_state(S {
            store,
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

async fn history(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, d)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.history(&a, &c, &d).await)
}
async fn send(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, d)): Path<(String, String)>,
    Json(v): Json<Send>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(s.store.send(&a, &c, &d, v).await)
}
pub async fn tool(
    pool: PgPool,
    mail: Option<std::sync::Arc<faktura_infrastructure::delivery::MailConfig>>,
    a: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::company::CompanyError as Error;
    let mut take = |key: &str| {
        args.remove(key)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| Error::Invalid(vec!["arguments".into()]))
    };
    let c = take("company_id")?;
    let d = take("id")?;
    let store = PgInboxDelivery { pool, mail };
    let result = match name {
        "inbox_delivery_history" => {
            if !args.is_empty() {
                return Err(Error::Invalid(vec!["arguments".into()]));
            }
            serde_json::to_value(store.history(a, &c, &d).await?)
        }
        "inbox_delivery_send" => serde_json::to_value(
            store
                .send(
                    a,
                    &c,
                    &d,
                    serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| Error::Invalid(vec!["arguments".into()]))?,
                )
                .await?,
        ),
        _ => return Err(Error::NotFound),
    };
    result.map_err(|_| Error::Unavailable)
}

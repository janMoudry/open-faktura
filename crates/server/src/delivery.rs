use crate::auth::{self, AuthState};
use crate::company::Failure;
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::delivery::{self, SendQuote};
use faktura_infrastructure::{auth::PgAuth, delivery::PgDelivery};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct DeliveryState {
    store: PgDelivery,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool, store: PgDelivery) -> Router {
    Router::new()
        .route("/api/companies/{company}/sales/{id}/pdf", get(pdf))
        .route(
            "/api/companies/{company}/sales/{id}/deliveries",
            get(history).post(send),
        )
        .route(
            "/api/companies/{company}/sales/{id}/deliveries/{delivery}/pdf",
            get(historical_pdf),
        )
        .layer(DefaultBodyLimit::max(1048576))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
        .with_state(DeliveryState {
            store,
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn actor(
    s: &DeliveryState,
    headers: &HeaderMap,
    write: bool,
) -> Result<String, auth::Failure> {
    if write {
        auth::same_origin(&s.auth, headers)?;
    }
    Ok(s.auth
        .store
        .session(&auth::token(headers, s.auth.secure))
        .await?
        .id)
}
fn response<T: serde::Serialize>(result: delivery::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => Failure(error).into_response(),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    #[serde(default = "first")]
    page: i64,
}
fn first() -> i64 {
    1
}
async fn history(
    State(s): State<DeliveryState>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Query(p): Query<Page>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(delivery::history(&s.store, &a, &c, &id, p.page).await)
}
async fn send(
    State(s): State<DeliveryState>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Json(cmd): Json<SendQuote>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(delivery::send(&s.store, &a, &c, &id, cmd).await)
}
fn pdf_response(v: delivery::Result<Vec<u8>>) -> Response {
    match v {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (
                    header::CONTENT_DISPOSITION,
                    "inline; filename=quotation.pdf",
                ),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn pdf(
    State(s): State<DeliveryState>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    pdf_response(delivery::pdf(&s.store, &a, &c, &id, None).await)
}
async fn historical_pdf(
    State(s): State<DeliveryState>,
    h: HeaderMap,
    Path((c, id, d)): Path<(String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    pdf_response(delivery::pdf(&s.store, &a, &c, &id, Some(&d)).await)
}

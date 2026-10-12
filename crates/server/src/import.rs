use crate::auth::{self, AuthState};
use crate::company::Failure;
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::import::{self as migration, Apply, ImportStore, Prepare};
use faktura_infrastructure::{auth::PgAuth, import::PgImport};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct ImportState {
    store: PgImport,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{company}/imports", get(list).post(prepare))
        .route(
            "/api/companies/{company}/imports/table",
            axum::routing::post(table),
        )
        .route(
            "/api/companies/{company}/imports/{id}",
            get(detail).post(apply),
        )
        .layer(DefaultBodyLimit::max(3_000_000))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
        .with_state(ImportState {
            store: PgImport(pool.clone()),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn actor(s: &ImportState, headers: &HeaderMap, write: bool) -> Result<String, auth::Failure> {
    if write {
        auth::same_origin(&s.auth, headers)?;
    }
    Ok(s.auth
        .store
        .session(&auth::token(headers, s.auth.secure))
        .await?
        .id)
}
fn response<T: serde::Serialize>(result: migration::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => Failure(error).into_response(),
    }
}
async fn list(State(s): State<ImportState>, h: HeaderMap, Path(company): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.list(&a, &company).await)
}
async fn detail(
    State(s): State<ImportState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.get(&a, &company, &id).await)
}
async fn prepare(
    State(s): State<ImportState>,
    h: HeaderMap,
    Path(company): Path<String>,
    body: std::result::Result<Json<Prepare>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let body = match body {
        Ok(Json(body)) => body,
        Err(_) => return response::<()>(Err(migration::Error::Invalid(vec!["file".into()]))),
    };
    response(migration::prepare(&s.store, &a, &company, body).await)
}
async fn apply(
    State(s): State<ImportState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    body: std::result::Result<Json<Apply>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let body = match body {
        Ok(Json(body)) => body,
        Err(_) => return response::<()>(Err(migration::Error::Invalid(vec!["file".into()]))),
    };
    response(migration::apply(&s.store, &a, &company, &id, body).await)
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    format: String,
    content: String,
    delimiter: String,
}
async fn table(
    State(s): State<ImportState>,
    h: HeaderMap,
    Path(company): Path<String>,
    body: std::result::Result<Json<Table>, axum::extract::rejection::JsonRejection>,
) -> Response {
    use base64::Engine;
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    let body = match body {
        Ok(Json(body)) => body,
        Err(_) => return response::<()>(Err(migration::Error::Invalid(vec!["file".into()]))),
    };
    let mut tx = match s.store.0.begin().await {
        Ok(t) => t,
        Err(_) => return response::<()>(Err(migration::Error::Unavailable)),
    };
    let allowed=sqlx::query_scalar::<_,String>("SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1::text::bigint AND m.user_id=$2::text::bigint AND c.deleted_at IS NULL").bind(&company).bind(&a).fetch_optional(&mut *tx).await;
    match allowed {
        Ok(Some(r)) if ["owner", "editor"].contains(&r.as_str()) => {}
        _ => return response::<()>(Err(migration::Error::Forbidden)),
    };
    drop(tx);
    let bytes = match base64::engine::general_purpose::STANDARD.decode(body.content) {
        Ok(b) => b,
        Err(_) => return response::<()>(Err(migration::Error::Invalid(vec!["file".into()]))),
    };
    static PARSERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let _permit = match PARSERS.try_acquire() {
        Ok(p) => p,
        Err(_) => return response::<()>(Err(migration::Error::Unavailable)),
    };
    response(
        tokio::task::spawn_blocking(move || {
            faktura_infrastructure::import_table::parse(&body.format, &bytes, &body.delimiter)
        })
        .await
        .unwrap_or(Err(migration::Error::Unavailable)),
    )
}

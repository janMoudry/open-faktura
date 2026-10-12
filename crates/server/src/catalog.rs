use crate::auth::{self, AuthState};
use crate::company::Failure;
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::catalog::{self, Create, Search, Update};
use faktura_infrastructure::{auth::PgAuth, catalog::PgCatalog};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct CatalogState {
    store: PgCatalog,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{company}/catalog/{id}/line", get(line))
        .route("/api/companies/{company}/catalog", get(list).post(create))
        .route(
            "/api/companies/{company}/catalog/{id}",
            get(detail).put(update),
        )
        .layer(DefaultBodyLimit::max(65536))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
        .with_state(CatalogState {
            store: PgCatalog(pool.clone()),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn actor(
    s: &CatalogState,
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
fn response<T: serde::Serialize>(result: catalog::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => Failure(error).into_response(),
    }
}
async fn list(
    State(s): State<CatalogState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Query(search): Query<Search>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(catalog::list(&s.store, &actor, &company, search).await)
}
async fn detail(
    State(s): State<CatalogState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(catalog::get(&s.store, &actor, &company, &id).await)
}
async fn create(
    State(s): State<CatalogState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(body): Json<Create>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(catalog::create(&s.store, &actor, &company, body).await)
}
async fn update(
    State(s): State<CatalogState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    Json(body): Json<Update>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(catalog::update(&s.store, &actor, &company, &id, body).await)
}

async fn line(
    State(s): State<CatalogState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    Query(options): Query<catalog::LineOptions>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(catalog::line(&s.store, &actor, &company, &id, options).await)
}

use crate::auth::{self, AuthState};
use crate::company::Failure;
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::contact::{self, Create, Search, Update};
use faktura_infrastructure::{auth::PgAuth, contact::PgContact};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct ContactState {
    store: PgContact,
    companies: faktura_infrastructure::company::PgCompany,
    ares: faktura_infrastructure::ares::Ares,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{company}/contacts/ares/{ico}",
            get(lookup_ares),
        )
        .route("/api/companies/{company}/contacts", get(list).post(create))
        .route(
            "/api/companies/{company}/contacts/{id}",
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
        .with_state(ContactState {
            store: PgContact(pool.clone()),
            companies: faktura_infrastructure::company::PgCompany(pool.clone()),
            ares: Default::default(),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn actor(
    s: &ContactState,
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
fn response<T: serde::Serialize>(result: contact::Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => Failure(error).into_response(),
    }
}
async fn list(
    State(s): State<ContactState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Query(search): Query<Search>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(contact::list(&s.store, &actor, &company, search).await)
}
async fn detail(
    State(s): State<ContactState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(contact::get(&s.store, &actor, &company, &id).await)
}
async fn create(
    State(s): State<ContactState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(body): Json<Create>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(contact::create(&s.store, &actor, &company, body).await)
}
async fn update(
    State(s): State<ContactState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    Json(body): Json<Update>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(contact::update(&s.store, &actor, &company, &id, body).await)
}

async fn lookup_ares(
    State(s): State<ContactState>,
    h: HeaderMap,
    Path((company, ico)): Path<(String, String)>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(faktura_application::ares::lookup(&s.companies, &s.ares, &actor, &company, &ico).await)
}

use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_application::share;
use faktura_infrastructure::{auth::PgAuth, share::PgShare};
use std::sync::Arc;
#[derive(Clone)]
struct ShareState {
    store: PgShare,
    auth: AuthState,
}
pub fn router(store: PgShare, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{company}/sales/{id}/shares",
            get(list).post(create),
        )
        .route(
            "/api/companies/{company}/sales/{id}/shares/{share}/revoke",
            post(revoke),
        )
        .route(
            "/api/companies/{company}/sales/{id}/shares/{share}/views",
            get(views),
        )
        .route("/api/public/quotes/resolve", post(resolve))
        .route("/api/public/quotes/pdf", post(pdf))
        .route("/api/public/quotes/view", post(view))
        .layer(DefaultBodyLimit::max(4096))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                let h = r.headers_mut();
                h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                h.insert(
                    header::REFERRER_POLICY,
                    HeaderValue::from_static("no-referrer"),
                );
                h.insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                );
                h.insert(
                    "x-robots-tag",
                    HeaderValue::from_static("noindex, nofollow, noarchive"),
                );
                r
            },
        ))
        .with_state(ShareState {
            auth: AuthState {
                store: Arc::new(PgAuth::new(store.pool.clone(), None)),
                origins,
                secure,
            },
            store,
        })
}
async fn actor(s: &ShareState, h: &HeaderMap, write: bool) -> Result<String, auth::Failure> {
    if write {
        auth::same_origin(&s.auth, h)?;
    }
    Ok(s.auth
        .store
        .session(&auth::token(h, s.auth.secure))
        .await?
        .id)
}
fn response<T: serde::Serialize>(v: share::Result<T>) -> Response {
    match v {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
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
async fn list(
    State(s): State<ShareState>,
    h: HeaderMap,
    Path((c, d)): Path<(String, String)>,
    Query(p): Query<Page>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(share::list(&s.store, &a, &c, &d, p.page).await)
}
async fn create(
    State(s): State<ShareState>,
    h: HeaderMap,
    Path((c, d)): Path<(String, String)>,
    Json(cmd): Json<share::Create>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(share::create(&s.store, &a, &c, &d, cmd).await)
}
async fn revoke(
    State(s): State<ShareState>,
    h: HeaderMap,
    Path((c, d, id)): Path<(String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(share::revoke(&s.store, &a, &c, &d, &id).await)
}
async fn views(
    State(s): State<ShareState>,
    h: HeaderMap,
    Path((c, d, id)): Path<(String, String, String)>,
    Query(p): Query<Page>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(share::views(&s.store, &a, &c, &d, &id, p.page).await)
}
// Tokens live in the URL fragment and POST body, never request paths/query strings/access logs.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    token: String,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Visit {
    token: String,
    visit_id: String,
    revision: i64,
}
async fn resolve(State(s): State<ShareState>, Json(cmd): Json<Token>) -> Response {
    response(share::public_quote(&s.store, &cmd.token).await)
}
async fn pdf(State(s): State<ShareState>, Json(cmd): Json<Token>) -> Response {
    match share::public_pdf(&s.store, &cmd.token).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (
                    header::CONTENT_DISPOSITION,
                    "inline; filename=quotation.pdf",
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn view(State(s): State<ShareState>, Json(cmd): Json<Visit>) -> Response {
    match share::view(&s.store, &cmd.token, &cmd.visit_id, cmd.revision).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

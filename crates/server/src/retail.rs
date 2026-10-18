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
use faktura_application::retail::{
    Command, Data, PostCommand, PostingInput, PostingStore, RetailStore, Search, UnpostCommand,
};
use faktura_infrastructure::{auth::PgAuth, retail::PgRetail};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/retail", get(list))
        .route("/api/companies/{c}/retail/preview", post(preview))
        .route(
            "/api/companies/{c}/retail/postings",
            get(postings).post(post_receipts),
        )
        .route(
            "/api/companies/{c}/retail/postings/preview",
            post(posting_preview),
        )
        .route("/api/companies/{c}/retail/postings/reverse", post(unpost))
        .route(
            "/api/companies/{c}/retail/postings/{id}/pdf",
            get(posting_pdf),
        )
        .route("/api/companies/{c}/retail/commands", post(command))
        .route("/api/companies/{c}/retail/{id}", get(detail))
        .route("/api/companies/{c}/retail/{id}/history", get(history))
        .route("/api/companies/{c}/retail/{id}/pdf", get(pdf))
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
fn response<T: serde::Serialize>(v: faktura_application::company::Result<T>) -> Response {
    match v {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn list(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).list(&a, &c, q).await)
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
    response(PgRetail(s.pool).get(&a, &c, &id).await)
}
async fn history(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).history(&a, &c, &id).await)
}
async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<Data>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).preview(&a, &c, d).await)
}
async fn command(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).execute(&a, &c, d).await)
}
async fn pdf(State(s): State<S>, h: HeaderMap, Path((c, id)): Path<(String, String)>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match PgRetail(s.pool).pdf(&a, &c, &id).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (header::CACHE_CONTROL, "no-store"),
                (header::CONTENT_DISPOSITION, "inline; filename=receipt.pdf"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

async fn posting_preview(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<PostingInput>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).posting_preview(&a, &c, d).await)
}

async fn post_receipts(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<PostCommand>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).post(&a, &c, d).await)
}

async fn unpost(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<UnpostCommand>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).unpost(&a, &c, d).await)
}

async fn postings(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgRetail(s.pool).postings(&a, &c).await)
}

#[derive(serde::Deserialize)]
struct PdfLanguage {
    language: String,
}
async fn posting_pdf(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Query(q): Query<PdfLanguage>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match PgRetail(s.pool).posting_pdf(&a, &c, &id, &q.language).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (header::CACHE_CONTROL, "no-store"),
                (header::CONTENT_DISPOSITION, "inline; filename=posting.pdf"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

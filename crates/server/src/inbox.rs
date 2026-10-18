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
use faktura_application::inbox::{Command, InboxStore, Search, Upload};
use faktura_infrastructure::{auth::PgAuth, inbox::PgInbox};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/inbox", get(list).post(upload))
        .route("/api/companies/{c}/inbox/commands", post(command))
        .route("/api/companies/{c}/inbox/{id}/file", get(download))
        .route("/api/companies/{c}/inbox/{id}/extract", get(extract))
        .route("/api/companies/{c}/inbox/{id}/history", get(history))
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
    response(PgInbox(s.pool).list(&a, &c, q).await)
}
async fn upload(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(d): Json<Upload>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgInbox(s.pool).upload(&a, &c, d).await)
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
    response(PgInbox(s.pool).execute(&a, &c, d).await)
}
async fn download(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match PgInbox(s.pool).download(&a, &c, &id).await {
        Ok((d, bytes)) => {
            let encoded = d
                .filename
                .as_bytes()
                .iter()
                .map(|b| format!("%{b:02X}"))
                .collect::<String>();
            (
                [
                    (header::CONTENT_TYPE, "application/octet-stream".to_string()),
                    (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
                    (header::CACHE_CONTROL, "no-store".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=download; filename*=UTF-8''{encoded}"),
                    ),
                ],
                bytes,
            )
                .into_response()
        }
        Err(e) => Failure(e).into_response(),
    }
}

async fn extract(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgInbox(s.pool).extract(&a, &c, &id).await)
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
    response(PgInbox(s.pool).history(&a, &c, &id).await)
}

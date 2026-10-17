use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use faktura_application::export::{self, Request};
use faktura_infrastructure::{auth::PgAuth, export::PgExport};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{company}/exports", post(download))
        .layer(DefaultBodyLimit::max(16384))
        .with_state(S {
            pool: pool.clone(),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn download(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(input): Json<Request>,
) -> Response {
    if let Err(e) = auth::same_origin(&s.auth, &h) {
        return e.into_response();
    }
    let actor = match s.auth.store.session(&auth::token(&h, s.auth.secure)).await {
        Ok(v) => v.id,
        Err(e) => return auth::Failure(e).into_response(),
    };
    match export::run(&PgExport(s.pool), &actor, &company, input).await {
        Ok(f) => (
            [
                (header::CONTENT_TYPE, f.media_type.to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", f.filename),
                ),
                (header::CACHE_CONTROL, "no-store".into()),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            ],
            f.bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

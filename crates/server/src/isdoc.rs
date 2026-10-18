use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use faktura_application::isdoc::IsdocStore;
use faktura_infrastructure::{auth::PgAuth, isdoc::PgIsdoc};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: std::sync::Arc<PgIsdoc>,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{company}/invoices/{id}/isdoc", get(read))
        .with_state(S {
            store: Arc::new(PgIsdoc(pool.clone())),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn read(
    State(s): State<S>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let actor = match s.auth.store.session(&auth::token(&h, s.auth.secure)).await {
        Ok(v) => v.id,
        Err(e) => return auth::Failure(e).into_response(),
    };
    match s.store.export(&actor, &company, &id).await {
        Ok(v) => (
            [
                (header::CACHE_CONTROL, "no-store".to_owned()),
                (header::CONTENT_TYPE, v.media_type.to_owned()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", v.filename),
                ),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
            ],
            v.bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

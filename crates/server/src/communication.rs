use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::communication::{CommunicationStore, Pause, Settings};
use faktura_infrastructure::{auth::PgAuth, communication::PgCommunication};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgCommunication,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{c}/communication", get(settings).put(save))
        .route(
            "/api/companies/{c}/invoices/{id}/communication/{purpose}",
            get(preview),
        )
        .route(
            "/api/companies/{c}/invoices/{id}/communication",
            axum::routing::put(pause),
        )
        .layer(DefaultBodyLimit::max(16384))
        .with_state(S {
            store: PgCommunication(pool.clone()),
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
async fn settings(State(s): State<S>, h: HeaderMap, Path(c): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.settings(&a, &c).await)
}
async fn save(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Json(v): Json<Settings>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.save(&a, &c, v).await)
}
async fn preview(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id, purpose)): Path<(String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.preview(&a, &c, &id, &purpose).await)
}
async fn pause(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, id)): Path<(String, String)>,
    Json(v): Json<Pause>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.pause(&a, &c, &id, v).await)
}

use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_application::smtp::{self, Save, SmtpStore, Test};
use faktura_infrastructure::{auth::PgAuth, smtp::PgSmtp};
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgSmtp,
    auth: AuthState,
}
pub fn router(store: PgSmtp, origins: Vec<String>, secure: bool) -> Router {
    let auth = AuthState {
        store: Arc::new(PgAuth::new(store.pool.clone(), None)),
        origins,
        secure,
    };
    Router::new()
        .route("/api/installation/smtp", get(read).put(save))
        .route("/api/installation/smtp/test", post(test))
        .layer(DefaultBodyLimit::max(16384))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                r
            },
        ))
        .with_state(S { store, auth })
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
fn response<T: serde::Serialize>(r: faktura_application::company::Result<T>) -> Response {
    match r {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn read(State(s): State<S>, h: HeaderMap) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.get(&a).await)
}
async fn save(State(s): State<S>, h: HeaderMap, Json(cmd): Json<Save>) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(smtp::save(&s.store, &a, cmd).await)
}
async fn test(State(s): State<S>, h: HeaderMap, Json(cmd): Json<Test>) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(smtp::test(&s.store, &a, cmd).await)
}

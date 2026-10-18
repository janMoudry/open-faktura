use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    Json, Router,
};
use faktura_application::document_output::{OutputStore, Request};
use faktura_infrastructure::{auth::PgAuth, document_output::PgOutput};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{company}/document-output",
            axum::routing::post(render),
        )
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
async fn render(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(request): Json<Request>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match PgOutput(s.pool).render(&a, &company, request).await {
        Ok(file) => (
            [
                (header::CONTENT_TYPE, file.media_type),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", file.filename),
                ),
                (header::CACHE_CONTROL, "no-store".into()),
            ],
            file.bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
pub async fn tool(
    pool: PgPool,
    actor: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use base64::Engine;
    use faktura_application::company::CompanyError;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let file = PgOutput(pool)
        .render(
            actor,
            &company,
            serde_json::from_value(serde_json::Value::Object(args)).map_err(|_| invalid())?,
        )
        .await?;
    Ok(
        serde_json::json!({"filename":file.filename,"media_type":file.media_type,"base64":base64::engine::general_purpose::STANDARD.encode(file.bytes)}),
    )
}

use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::document_exports::{ExportStore, Request, Target, Unlock};
use faktura_infrastructure::{auth::PgAuth, document_exports::PgDocumentExports};
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
            "/api/companies/{company}/document-exports",
            get(read).post(create),
        )
        .route(
            "/api/companies/{company}/document-exports/unlock",
            axum::routing::post(unlock),
        )
        .route(
            "/api/companies/{company}/document-exports/{id}",
            get(download),
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
fn response<T: serde::Serialize>(v: faktura_application::company::Result<T>) -> Response {
    match v {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

fn download_response(
    value: faktura_application::company::Result<faktura_application::document_exports::File>,
) -> Response {
    match value {
        Ok(file) => (
            [
                (header::CONTENT_TYPE, "application/json".into()),
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
async fn read(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Query(target): Query<Target>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(
        PgDocumentExports(s.pool)
            .history(&a, &company, target)
            .await,
    )
}
async fn create(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(request): Json<Request>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    download_response(
        PgDocumentExports(s.pool)
            .create(&a, &company, request)
            .await,
    )
}
async fn download(
    State(s): State<S>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    download_response(PgDocumentExports(s.pool).file(&a, &company, &id).await)
}
async fn unlock(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(command): Json<Unlock>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(
        PgDocumentExports(s.pool)
            .unlock(&a, &company, command)
            .await,
    )
}
pub async fn tool(
    pool: PgPool,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use base64::Engine;
    use faktura_application::company::CompanyError;
    let invalid = || CompanyError::Invalid(vec!["arguments".into()]);
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(invalid)?;
    let store = PgDocumentExports(pool);
    match name {
        "document_export_history" => serde_json::to_value(
            store
                .history(
                    actor,
                    &company,
                    serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| invalid())?,
                )
                .await?,
        )
        .map_err(|_| CompanyError::Unavailable),
        "document_export_unlock" => serde_json::to_value(
            store
                .unlock(
                    actor,
                    &company,
                    serde_json::from_value(serde_json::Value::Object(args))
                        .map_err(|_| invalid())?,
                )
                .await?,
        )
        .map_err(|_| CompanyError::Unavailable),
        _ => {
            let file = if name == "document_export" {
                store
                    .create(
                        actor,
                        &company,
                        serde_json::from_value(serde_json::Value::Object(args))
                            .map_err(|_| invalid())?,
                    )
                    .await?
            } else if name == "document_export_file" {
                let id = args
                    .remove("id")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or_else(invalid)?;
                if !args.is_empty() {
                    return Err(invalid());
                }
                store.file(actor, &company, &id).await?
            } else {
                return Err(invalid());
            };
            Ok(
                serde_json::json!({"filename":file.filename,"media_type":"application/json","base64":base64::engine::general_purpose::STANDARD.encode(file.bytes)}),
            )
        }
    }
}

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
use faktura_application::company_admin::{AdministrationStore, ArchiveStore, Command};
use faktura_infrastructure::{auth::PgAuth, company_admin::PgAdministration};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    pool: PgPool,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/trash", get(trash))
        .route("/api/companies/{company}/archive", get(export))
        .route(
            "/api/companies/{company}/administration",
            get(read).post(write),
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

async fn read(State(s): State<S>, h: HeaderMap, Path(company): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgAdministration(s.pool).list(&a, &company).await)
}
async fn trash(State(s): State<S>, h: HeaderMap) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgAdministration(s.pool).trash(&a).await)
}
async fn write(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(d): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgAdministration(s.pool).execute(&a, &company, d).await)
}

async fn export(State(s): State<S>, h: HeaderMap, Path(company): Path<String>) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match PgAdministration(s.pool).export(&a, &company).await {
        Ok(v) => (
            [
                (header::CONTENT_TYPE, "application/zip".into()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", v.file_name),
                ),
                (header::CACHE_CONTROL, "no-store".into()),
            ],
            v.bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

/// Shared MCP dispatcher; HTTP and MCP use exactly the same stores.
pub async fn tool(
    pool: PgPool,
    actor: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use base64::Engine;
    use faktura_application::company::CompanyError;
    let store = PgAdministration(pool);
    if name == "company_trash" {
        if !args.is_empty() {
            return Err(CompanyError::Invalid(vec!["arguments".into()]));
        }
        return serde_json::to_value(store.trash(actor).await?)
            .map_err(|_| CompanyError::Unavailable);
    }
    let company = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or(CompanyError::Invalid(vec!["company_id".into()]))?;
    match name {
        "company_administration" => {
            if !args.is_empty() {
                return Err(CompanyError::Invalid(vec!["arguments".into()]));
            }
            serde_json::to_value(store.list(actor, &company).await?)
                .map_err(|_| CompanyError::Unavailable)
        }
        "company_administration_execute" => {
            let command = serde_json::from_value(serde_json::Value::Object(args))
                .map_err(|_| CompanyError::Invalid(vec!["arguments".into()]))?;
            serde_json::to_value(store.execute(actor, &company, command).await?)
                .map_err(|_| CompanyError::Unavailable)
        }
        "company_archive" => {
            if !args.is_empty() {
                return Err(CompanyError::Invalid(vec!["arguments".into()]));
            }
            let file = store.export(actor, &company).await?;
            Ok(
                serde_json::json!({"filename":file.file_name,"media_type":"application/zip","base64":base64::engine::general_purpose::STANDARD.encode(file.bytes)}),
            )
        }
        _ => Err(CompanyError::Invalid(vec!["tool".into()])),
    }
}

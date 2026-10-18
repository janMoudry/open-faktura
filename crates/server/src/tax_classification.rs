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
use faktura_application::tax_classification::{Command, TaxClassificationStore};
use faktura_infrastructure::{auth::PgAuth, tax_classification::PgTaxClassification};
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
            "/api/companies/{company}/reports/tax/classifications",
            get(summary),
        )
        .route(
            "/api/companies/{company}/reports/tax/classifications/{kind}/{id}",
            get(read).post(write),
        )
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
fn response<T: serde::Serialize>(value: faktura_application::company::Result<T>) -> Response {
    match value {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn read(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, id)): Path<(String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTaxClassification(s.pool).get(&a, &c, &k, &id).await)
}
async fn write(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, id)): Path<(String, String, String)>,
    Json(input): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(
        PgTaxClassification(s.pool)
            .set(&a, &c, &k, &id, input)
            .await,
    )
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    year: i32,
}
async fn summary(
    State(s): State<S>,
    h: HeaderMap,
    Path(c): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTaxClassification(s.pool).summary(&a, &c, q.year).await)
}
pub async fn tool(
    pool: PgPool,
    a: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::company::CompanyError as Error;
    let text = |args: &mut serde_json::Map<String, serde_json::Value>,
                key: &str|
     -> Result<String, Error> {
        args.remove(key)
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| Error::Invalid(vec!["arguments".into()]))
    };
    let c = text(&mut args, "company_id")?;
    let store = PgTaxClassification(pool);
    match name {
        "tax_classification_summary" => {
            let year = args
                .remove("year")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok())
                .ok_or_else(|| Error::Invalid(vec!["year".into()]))?;
            if !args.is_empty() {
                return Err(Error::Invalid(vec!["arguments".into()]));
            }
            serde_json::to_value(store.summary(a, &c, year).await?).map_err(|_| Error::Unavailable)
        }
        "tax_classification_get" | "tax_classification_set" => {
            let kind = text(&mut args, "kind")?;
            let id = text(&mut args, "id")?;
            if name == "tax_classification_get" {
                if !args.is_empty() {
                    return Err(Error::Invalid(vec!["arguments".into()]));
                }
                serde_json::to_value(store.get(a, &c, &kind, &id).await?)
                    .map_err(|_| Error::Unavailable)
            } else {
                let command = serde_json::from_value(serde_json::Value::Object(args))
                    .map_err(|_| Error::Invalid(vec!["arguments".into()]))?;
                serde_json::to_value(store.set(a, &c, &kind, &id, command).await?)
                    .map_err(|_| Error::Unavailable)
            }
        }
        _ => Err(Error::Invalid(vec!["arguments".into()])),
    }
}

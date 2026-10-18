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
use faktura_application::taxpayer_filing_profile::{Command, TaxpayerFilingProfileStore};
use faktura_infrastructure::{auth::PgAuth, taxpayer_filing_profile::PgTaxpayerFilingProfile};
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
            "/api/companies/{company}/reports/tax/filing-profile",
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
fn response<T: serde::Serialize>(v: faktura_application::company::Result<T>) -> Response {
    match v {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

async fn read(State(s): State<S>, Path(c): Path<String>, h: HeaderMap) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTaxpayerFilingProfile(s.pool).get(&a, &c).await)
}
async fn write(
    State(s): State<S>,
    Path(c): Path<String>,
    h: HeaderMap,
    Json(d): Json<Command>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(PgTaxpayerFilingProfile(s.pool).set(&a, &c, d).await)
}

pub async fn tool(
    pool: PgPool,
    a: &str,
    name: &str,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> faktura_application::company::Result<serde_json::Value> {
    use faktura_application::company::CompanyError as Error;
    let c = args
        .remove("company_id")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| Error::Invalid(vec!["arguments".into()]))?;
    let store = PgTaxpayerFilingProfile(pool);
    let result = if name == "taxpayer_filing_profile_get" {
        if !args.is_empty() {
            return Err(Error::Invalid(vec!["arguments".into()]));
        }
        store.get(a, &c).await?
    } else if name == "taxpayer_filing_profile_set" {
        store
            .set(
                a,
                &c,
                serde_json::from_value(serde_json::Value::Object(args))
                    .map_err(|_| Error::Invalid(vec!["arguments".into()]))?,
            )
            .await?
    } else {
        return Err(Error::NotFound);
    };
    serde_json::to_value(result).map_err(|_| Error::Unavailable)
}

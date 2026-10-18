use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::exchange_rate::RateProvider;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, exchange_rate::Cnb};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    companies: PgCompany,
    auth: AuthState,
    provider: Arc<dyn RateProvider>,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    router_with_provider(pool, origins, secure, Arc::new(Cnb::default()))
}
pub fn router_with_provider(
    pool: PgPool,
    origins: Vec<String>,
    secure: bool,
    provider: Arc<dyn RateProvider>,
) -> Router {
    Router::new()
        .route("/api/companies/{company}/exchange-rates", get(read))
        .with_state(S {
            companies: PgCompany(pool.clone()),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
            provider,
        })
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    currency: String,
    date: String,
}
async fn read(
    State(s): State<S>,
    h: HeaderMap,
    Path(company): Path<String>,
    Query(q): Query<Search>,
) -> Response {
    let actor = match s.auth.store.session(&auth::token(&h, s.auth.secure)).await {
        Ok(v) => v.id,
        Err(e) => return auth::Failure(e).into_response(),
    };
    match faktura_application::exchange_rate::lookup(
        &s.companies,
        s.provider.as_ref(),
        &actor,
        &company,
        &q.currency,
        &q.date,
    )
    .await
    {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

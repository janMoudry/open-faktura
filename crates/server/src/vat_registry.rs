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
use faktura_application::vat_registry::VatRegistry;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, vat_registry::Rws};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    companies: PgCompany,
    auth: AuthState,
    provider: Arc<dyn VatRegistry>,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    router_with_provider(pool, origins, secure, Arc::new(Rws::default()))
}
pub fn router_with_provider(
    pool: PgPool,
    origins: Vec<String>,
    secure: bool,
    provider: Arc<dyn VatRegistry>,
) -> Router {
    Router::new()
        .route("/api/companies/{company}/vat-registry", get(read))
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
    vat_number: String,
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
    match faktura_application::vat_registry::lookup(
        &s.companies,
        s.provider.as_ref(),
        &actor,
        &company,
        &q.vat_number,
    )
    .await
    {
        Ok(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use faktura_domain::invoice::Calculation;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct InvoiceState {
    payments: faktura_infrastructure::payment::PgPayment,
    store: PgCompany,
    drafts: faktura_infrastructure::draft::PgDraft,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route("/api/companies/{company}/overview", get(overview))
        .route(
            "/api/companies/{company}/invoices/{id}/payments",
            get(payments).post(pay),
        )
        .route(
            "/api/companies/{company}/invoices/{id}/payments/{payment}/reverse",
            post(reverse_payment),
        )
        .route("/api/companies/{company}/invoices", get(list).post(create))
        .route(
            "/api/companies/{company}/invoices/{id}",
            get(detail).put(update),
        )
        .route(
            "/api/companies/{company}/invoices/calculate",
            post(calculate),
        )
        .route("/api/companies/{company}/invoices/{id}/issue", post(issue))
        .route("/api/companies/{company}/invoices/{id}/pdf", get(pdf))
        .layer(DefaultBodyLimit::max(2_097_152))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                r
            },
        ))
        .with_state(InvoiceState {
            payments: faktura_infrastructure::payment::PgPayment(pool.clone()),
            store: PgCompany(pool.clone()),
            drafts: faktura_infrastructure::draft::PgDraft(pool.clone()),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn calculate(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(input): Json<Calculation>,
) -> Response {
    if let Err(e) = auth::same_origin(&s.auth, &h) {
        return e.into_response();
    }
    let actor = match s.auth.store.session(&auth::token(&h, s.auth.secure)).await {
        Ok(v) => v.id,
        Err(e) => return auth::Failure(e).into_response(),
    };
    match faktura_application::invoice::calculate(&s.store, &actor, &company, input).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

use faktura_application::draft::{self, Create, Search, Update};
async fn actor(s: &InvoiceState, h: &HeaderMap, write: bool) -> Result<String, auth::Failure> {
    if write {
        auth::same_origin(&s.auth, h)?;
    }
    Ok(s.auth
        .store
        .session(&auth::token(h, s.auth.secure))
        .await?
        .id)
}
fn response<T: serde::Serialize>(result: draft::Result<T>) -> Response {
    match result {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn list(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Query(search): Query<Search>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(draft::list(&s.drafts, &actor, &company, search).await)
}
async fn detail(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(draft::get(&s.drafts, &actor, &company, &id).await)
}
async fn create(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path(company): Path<String>,
    Json(body): Json<Create>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(draft::create(&s.drafts, &actor, &company, body).await)
}
async fn update(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    Json(body): Json<Update>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(draft::update(&s.drafts, &actor, &company, &id, body).await)
}

async fn issue(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    Json(body): Json<draft::Issue>,
) -> Response {
    let actor = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(draft::issue(&s.drafts, &actor, &company, &id, body).await)
}
async fn pdf(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let actor = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    match draft::pdf(&s.drafts, &actor, &company, &id).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (header::CONTENT_DISPOSITION, "inline; filename=invoice.pdf"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => Failure(e).into_response(),
    }
}

async fn payments(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(faktura_application::payment::get(&s.payments, &a, &company, &id).await)
}
async fn pay(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id)): Path<(String, String)>,
    Json(cmd): Json<faktura_application::payment::Register>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(faktura_application::payment::register(&s.payments, &a, &company, &id, cmd).await)
}
async fn reverse_payment(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path((company, id, payment)): Path<(String, String, String)>,
    Json(cmd): Json<faktura_application::payment::Reverse>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(
        faktura_application::payment::reverse(&s.payments, &a, &company, &id, &payment, cmd).await,
    )
}
async fn overview(
    State(s): State<InvoiceState>,
    h: HeaderMap,
    Path(company): Path<String>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    response(faktura_application::payment::overview(&s.payments, &a, &company).await)
}

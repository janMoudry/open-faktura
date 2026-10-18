use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use faktura_domain::Availability;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route(
            "/api/health",
            get(|| async { Json(Health { status: "ok" }) }),
        )
        .route("/api/ready", get(ready))
        .with_state(pool)
}

async fn ready(State(pool): State<PgPool>) -> (StatusCode, Json<Health>) {
    match faktura_application::readiness(faktura_infrastructure::database_available(&pool).await) {
        Availability::Ready => (StatusCode::OK, Json(Health { status: "ready" })),
        Availability::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(Health {
                status: "unavailable",
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn liveness_works_without_database_but_readiness_fails() {
        let pool = faktura_infrastructure::pool("postgres://test:test@127.0.0.1:1/test").unwrap();
        let app = router(pool);
        let live = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(live.status(), StatusCode::OK);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
            br#"{"status":"unavailable"}"#
        );
    }
}

pub mod auth;

pub mod company;
pub mod mcp;

pub mod contact;
pub mod invoice;

pub mod import;

pub mod catalog;

pub mod sales;

pub mod delivery;

pub mod share;

pub mod invoice_delivery;

pub mod smtp;

pub mod attachment;

pub mod export;

pub mod advance;

pub mod tax_receipt;

pub mod advance_settlement;

pub mod credit_note;

pub mod recurring;

pub mod communication;

pub mod invoice_share;

pub mod received;

pub mod banking;

pub mod cash;

pub mod retail;

pub mod inbox;

pub mod stock;

pub mod preferences;

pub mod period_report;

pub mod exchange_rate;

pub mod company_admin;

pub mod isdoc;

pub mod vat_registry;

pub mod document_tags;

pub mod bank_notification;

pub mod dashboard_widgets;

pub mod document_output;

pub mod document_style;

pub mod draft_trash;

pub mod inbox_mailbox;

pub mod vies;

pub mod document_exports;

pub mod tax_classification;

pub mod taxpayer_filing_profile;

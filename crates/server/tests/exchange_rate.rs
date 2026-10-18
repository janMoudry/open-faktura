use faktura_application::{auth, company};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        pool.clone(),
        Some("synthetic-catalog-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-catalog-bootstrap-key-only-tests",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (actor, token) = auth::login(&auth, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let first = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        KEY,
        CompanySettings {
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..CompanySettings::default()
        },
    )
    .await
    .unwrap();
    let second = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        "00000000-0000-4000-8000-000000000002",
        CompanySettings {
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..CompanySettings::default()
        },
    )
    .await
    .unwrap();
    (actor.id, token, first.id, second.id)
}

struct Fixture(std::sync::atomic::AtomicUsize);
#[async_trait::async_trait]
impl faktura_application::exchange_rate::RateProvider for Fixture {
    async fn lookup(
        &self,
        currency: &str,
        date: sqlx::types::chrono::NaiveDate,
    ) -> faktura_application::company::Result<faktura_application::exchange_rate::Rate> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        faktura_infrastructure::exchange_rate::parse(
            "16.10.2026 #200\nzemě|měna|množství|kód|kurz\nEMU|euro|1|EUR|25,125\n",
            currency,
            date,
        )
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn read_only_rate_rest_mcp_scope_and_validation(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use std::sync::{atomic::Ordering, Arc};
    use tower::ServiceExt;
    let (a, token, c, _) = setup(&pool).await;
    let provider = Arc::new(Fixture(std::sync::atomic::AtomicUsize::new(0)));
    let app = faktura_server::exchange_rate::router_with_provider(
        pool.clone(),
        vec![],
        false,
        provider.clone(),
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/exchange-rates?currency=EUR&date=2026-10-17"
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["exchange_rate"]["rate"], "25.125");
    assert_eq!(value["published_on"], "2026-10-16");
    let calls = provider.0.load(Ordering::SeqCst);
    for (company, currency, date) in [
        ("999999", "EUR", "2026-10-17"),
        (&c, "XXX", "2026-10-17"),
        (&c, "EUR", "bad"),
        (&c, "CZK", "2026-10-17"),
    ] {
        assert!(faktura_application::exchange_rate::lookup(
            &PgCompany(pool.clone()),
            provider.as_ref(),
            &a,
            company,
            currency,
            date
        )
        .await
        .is_err());
    }
    assert_eq!(provider.0.load(Ordering::SeqCst), calls);
    let unauthorized = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/exchange-rates?currency=EUR&date=2026-10-17"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), 401);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token)
        .with_rate_provider(provider.clone());
    let (client_io, server_io) = tokio::io::duplex(65536);
    let task = tokio::spawn(async move {
        server
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ().serve(client_io).await.unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("exchange_rate_lookup").with_arguments(
                serde_json::json!({"company_id":c,"currency":"EUR","date":"2026-10-17"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let output = serde_json::to_value(result).unwrap().to_string();
    assert!(output.contains("25.125"));
    client.cancel().await.unwrap();
    task.await.unwrap();
    let documents: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(documents, 0);
}

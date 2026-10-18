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
impl faktura_application::vies::ViesRegistry for Fixture {
    async fn lookup(
        &self,
        vat_number: &str,
    ) -> company::Result<faktura_application::vies::Observation> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        use faktura_application::vies::*;
        Ok(Observation {
            vat_number: vat_number.into(),
            valid: false,
            requested_on: "2026-10-18".into(),
            checked_at: "2026-10-18T00:00:00Z".into(),
            name: String::new(),
            address: String::new(),
            source: "EU_VIES".into(),
        })
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn authorization_before_provider_and_rest(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use std::sync::{atomic::Ordering, Arc};
    use tower::ServiceExt;
    let (actor, token, c, _) = setup(&pool).await;
    let provider = Arc::new(Fixture(std::sync::atomic::AtomicUsize::new(0)));
    for (company, value) in [
        ("999999", "DE123456789"),
        (&c, "GB12345678"),
        (&c, "DE<xml>"),
        (&c, "GB123456789901"),
    ] {
        assert!(faktura_application::vies::lookup(
            &PgCompany(pool.clone()),
            provider.as_ref(),
            &actor,
            company,
            value
        )
        .await
        .is_err());
    }
    assert_eq!(provider.0.load(Ordering::SeqCst), 0);
    let app =
        faktura_server::vies::router_with_provider(pool.clone(), vec![], false, provider.clone());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/vies?vat_number=DE123456789"))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["valid"], false);
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/vies?vat_number=DE123456789"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token)
        .with_vies_provider(provider.clone());
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
    let listed = client.list_tools(None).await.unwrap();
    for name in [
        "vies_lookup",
        "vat_registry_lookup",
        "invoice_isdoc",
        "tax_evidence_export",
        "dashboard_widgets_get",
    ] {
        assert!(
            listed.tools.iter().any(|t| t.name == name),
            "missing tool {name}"
        );
    }
    let output = client
        .call_tool(
            CallToolRequestParams::new("vies_lookup").with_arguments(
                serde_json::json!({"company_id":c,"vat_number":"DE123456789"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(output.is_error, Some(true));
    assert!(serde_json::to_value(output)
        .unwrap()
        .to_string()
        .contains("EU_VIES"));
    assert_eq!(provider.0.load(Ordering::SeqCst), 2);
    client.cancel().await.unwrap();
    task.await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM contact")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

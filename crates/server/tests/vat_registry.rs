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
impl faktura_application::vat_registry::VatRegistry for Fixture {
    async fn lookup(
        &self,
        vat_number: &str,
    ) -> company::Result<faktura_application::vat_registry::Observation> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        use faktura_application::vat_registry::*;
        Ok(Observation {
            vat_number: vat_number.into(),
            subject_type: SubjectType::NotFound,
            reliability: Reliability::NotFound,
            accounts: vec![],
            source: "CZ_RWS".into(),
            generated_on: "2026-10-18".into(),
            checked_at: "2026-10-18T00:00:00Z".into(),
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
        ("999999", "CZ12345678"),
        (&c, "DE12345678"),
        (&c, "CZ<xml>"),
        (&c, "CZ12345678901"),
    ] {
        assert!(faktura_application::vat_registry::lookup(
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
    let app = faktura_server::vat_registry::router_with_provider(
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
                    "/api/companies/{c}/vat-registry?vat_number=CZ12345678"
                ))
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
    assert_eq!(value["subject_type"], "not_found");
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/vat-registry?vat_number=CZ12345678"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    assert_eq!(provider.0.load(Ordering::SeqCst), 1);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM contact")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

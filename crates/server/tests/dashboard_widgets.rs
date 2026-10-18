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

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn own_account_layout_revision_replay_isolation_and_rest(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use faktura_application::dashboard_widgets::*;
    use faktura_infrastructure::dashboard_widgets::PgDashboardWidgets;
    use tower::ServiceExt;
    let (actor, token, _, _) = setup(&pool).await;
    let other:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('widget-other@example.test','no-login') RETURNING id").fetch_one(&pool).await.unwrap();
    let store = PgDashboardWidgets(pool.clone());
    assert_eq!(store.get(&actor).await.unwrap(), Layout::default());
    let input = Layout {
        revision: 0,
        widgets: vec![Widget {
            kind: Kind::Tax,
            variant: Variant::Compact,
        }],
    };
    let (a, b) = tokio::join!(
        store.set(&actor, input.clone()),
        store.set(&actor, input.clone())
    );
    let saved = a.unwrap();
    assert_eq!(saved, b.unwrap());
    assert_eq!(saved.revision, 1);
    assert_eq!(
        store.get(&other.to_string()).await.unwrap(),
        Layout::default()
    );
    assert!(store
        .set(
            &actor,
            Layout {
                revision: 0,
                widgets: vec![]
            }
        )
        .await
        .is_err());
    assert!(store
        .set(
            &actor,
            Layout {
                revision: 1,
                widgets: vec![saved.widgets[0].clone(), saved.widgets[0].clone()]
            }
        )
        .await
        .is_err());
    let app = faktura_server::dashboard_widgets::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/dashboard-widgets")
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let actual: Layout =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(actual, saved);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/dashboard-widgets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let changed = store
        .set(
            &actor,
            Layout {
                revision: 1,
                widgets: vec![],
            },
        )
        .await
        .unwrap();
    assert!(changed.widgets.is_empty());
    assert_eq!(store.get(&actor).await.unwrap().revision, 2);
}

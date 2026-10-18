use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use faktura_application::{
    auth::{self, AuthStore},
    company::{self, CompanyError, CompanyStore},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use sqlx::PgPool;
use tower::ServiceExt;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
async fn user(pool: &PgPool, email: &str, admin: bool) -> String {
    sqlx::query_scalar::<_,i64>("INSERT INTO app_user(email,password_hash,installation_admin) VALUES ($1,'test-only-no-login',$2) RETURNING id").bind(email).bind(admin).fetch_one(pool).await.unwrap().to_string()
}
fn complete() -> CompanySettings {
    CompanySettings {
        name: "Synthetic company".into(),
        street: "Test 1".into(),
        city: "Praha".into(),
        postal_code: "11000".into(),
        tax_effective_from: "2026-01-01".into(),
        ..Default::default()
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn companies_are_isolated_and_revision_writes_are_atomic(pool: PgPool) {
    let a = user(&pool, "a@example.test", false).await;
    let b = user(&pool, "b@example.test", true).await;
    let store = PgCompany(pool.clone());
    let c = company::create(&store, &a, KEY, CompanySettings::default())
        .await
        .unwrap();
    let d = company::create(&store, &b, KEY, CompanySettings::default())
        .await
        .unwrap();
    assert_ne!(c.id, d.id);
    assert_eq!(company::list(&store, &b).await.unwrap().len(), 1);
    assert!(matches!(
        company::context(&store, &b, &c.id).await,
        Err(CompanyError::NotFound)
    ));
    assert!(matches!(
        company::update(&store, &b, &c.id, 1, 0, false, complete()).await,
        Err(CompanyError::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES ($1,$2,'viewer')")
        .bind(c.id.parse::<i64>().unwrap())
        .bind(b.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        company::context(&store, &b, &c.id).await.unwrap().role,
        "viewer"
    );
    assert!(matches!(
        company::update(&store, &b, &c.id, 1, 0, false, complete()).await,
        Err(CompanyError::Forbidden)
    ));
    let (left, right) = tokio::join!(
        company::update(&store, &a, &c.id, 1, 3, true, complete()),
        company::update(&store, &a, &c.id, 1, 3, true, complete())
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert!(
        matches!(left, Err(CompanyError::Conflict)) || matches!(right, Err(CompanyError::Conflict))
    );
    let saved = company::context(&store, &a, &c.id).await.unwrap();
    assert!(saved.completed);
    assert!(saved.can_issue);
    assert_eq!(saved.tax_history.len(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM company_audit WHERE company_id=$1")
            .bind(c.id.parse::<i64>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    let mut next = saved.settings.clone();
    next.tax_mode = "vat_payer".into();
    next.vat_number = "CZ12345678".into();
    next.tax_effective_from = "2026-10-01".into();
    let changed = company::update(&store, &a, &c.id, saved.revision, 3, true, next)
        .await
        .unwrap();
    assert_eq!(changed.tax_history.len(), 2);
    assert_eq!(changed.tax_history[0].mode, "non_payer");
    sqlx::query("DELETE FROM membership WHERE company_id=$1 AND user_id=$2")
        .bind(c.id.parse::<i64>().unwrap())
        .bind(b.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        company::context(&store, &b, &c.id).await,
        Err(CompanyError::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn request_replay_and_incomplete_profile(pool: PgPool) {
    let a = user(&pool, "a@example.test", true).await;
    let store = PgCompany(pool.clone());
    let (x, y) = tokio::join!(
        company::create(&store, &a, KEY, CompanySettings::default()),
        company::create(&store, &a, KEY, CompanySettings::default())
    );
    let c = x.unwrap();
    assert_eq!(c.id, y.unwrap().id);
    assert_eq!(store.list(&a).await.unwrap().len(), 1);
    assert!(matches!(
        company::create(&store, &a, KEY, complete()).await,
        Err(CompanyError::Conflict)
    ));
    assert!(matches!(
        company::update(&store, &a, &c.id, 1, 3, true, CompanySettings::default()).await,
        Err(CompanyError::Invalid(_))
    ));
    assert_eq!(store.get(&a, &c.id).await.unwrap().revision, 1);
    let s = company::update(&store, &a, &c.id, 1, 2, false, CompanySettings::default())
        .await
        .unwrap();
    assert_eq!(
        PgCompany(pool.clone()).get(&a, &c.id).await.unwrap().step,
        2
    );
    assert!(!s.completed);
}
async fn login(pool: &PgPool) -> (String, String) {
    let s = PgAuth::new(
        pool.clone(),
        Some("synthetic-bootstrap-company-secret-12345".into()),
    );
    auth::bootstrap(
        &s,
        "synthetic-bootstrap-company-secret-12345",
        "admin@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (u, t) = auth::login(&s, "admin@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    (u.id, t)
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn rest_requires_session_origin_and_membership(pool: PgPool) {
    let (a, token) = login(&pool).await;
    let b = user(&pool, "b@example.test", false).await;
    let store = PgCompany(pool.clone());
    let other = company::create(&store, &b, KEY, complete()).await.unwrap();
    let app = faktura_server::company::router(pool.clone(), vec!["https://app.test".into()], false);
    for (cookie, path, expected) in [
        (false, "/api/companies".into(), StatusCode::UNAUTHORIZED),
        (
            true,
            format!("/api/companies/{}", other.id),
            StatusCode::NOT_FOUND,
        ),
        (true, "/api/companies/99999".into(), StatusCode::NOT_FOUND),
    ] {
        let mut req = Request::builder().uri(path);
        if cookie {
            req = req.header("cookie", format!("faktura_session={token}"));
        }
        let response = app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    let body = serde_json::json!({"request_id":KEY,"settings":complete()});
    for (origin, expected) in [
        ("https://evil.test", StatusCode::UNAUTHORIZED),
        ("https://app.test", StatusCode::OK),
    ] {
        let req = Request::builder()
            .uri("/api/companies")
            .method("POST")
            .header("cookie", format!("faktura_session={token}"))
            .header("origin", origin)
            .header("x-faktura-request", "1")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/companies")
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["id"], store.list(&a).await.unwrap()[0].id);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn mcp_sdk_client_obeys_membership_and_revocation(pool: PgPool) {
    use rmcp::{model::CallToolRequestParams, ServiceExt};
    let (a, token) = login(&pool).await;
    let b = user(&pool, "b@example.test", false).await;
    let store = PgCompany(pool.clone());
    let own = company::create(&store, &a, KEY, complete()).await.unwrap();
    let other = company::create(&store, &b, KEY, complete()).await.unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token.clone());
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
    let tools = client.list_all_tools().await.unwrap();
    for required in ["companies_list", "company_context", "invoice_calculate"] {
        assert!(
            tools.iter().any(|tool| tool.name.as_ref() == required),
            "missing {required}"
        );
    }
    let names: std::collections::HashSet<_> = tools.iter().map(|tool| tool.name.as_ref()).collect();
    assert_eq!(names.len(), tools.len());
    let listed = client
        .call_tool(CallToolRequestParams::new("companies_list"))
        .await
        .unwrap();
    let text = serde_json::to_string(&listed).unwrap();
    assert!(text.contains(&format!("\\\"id\\\":\\\"{}\\\"", own.id)));
    for (id, error) in [(&own.id, false), (&other.id, true)] {
        let args = serde_json::json!({"company_id":id})
            .as_object()
            .unwrap()
            .clone();
        let response = client
            .call_tool(CallToolRequestParams::new("company_context").with_arguments(args))
            .await
            .unwrap();
        assert_eq!(response.is_error, Some(error));
    }
    // The same preview crosses REST and the real MCP SDK without numeric coercion.
    let input = serde_json::json!({"calculation_version":"of-czk-v1","currency":"CZK","tax_mode":"vat_payer","price_mode":"net","discount_percent":"0","rounding":"none","lines":[{"id":"a","description":"Test","unit":"ks","quantity":"3","unit_price":"19.995","discount_percent":"0","vat_percent":"21"}]});
    let router = faktura_server::invoice::router(
        pool.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    for (id, cookie, expected) in [
        (&own.id, token.as_str(), 200),
        (&other.id, token.as_str(), 404),
        (&own.id, "", 401),
    ] {
        use axum::{
            body::{to_bytes, Body},
            http::Request,
        };
        use tower::ServiceExt;
        let result = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/companies/{id}/invoices/calculate"))
                    .header("origin", "https://app.example.test")
                    .header("x-faktura-request", "1")
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("content-type", "application/json")
                    .body(Body::from(input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status().as_u16(), expected);
        if expected == 200 {
            let rest: serde_json::Value =
                serde_json::from_slice(&to_bytes(result.into_body(), 65536).await.unwrap())
                    .unwrap();
            assert_eq!(rest["payable"], "72.59");
            let mut args = input.as_object().unwrap().clone();
            args.insert("company_id".into(), own.id.clone().into());
            let result = client
                .call_tool(CallToolRequestParams::new("invoice_calculate").with_arguments(args))
                .await
                .unwrap();
            assert_eq!(result.is_error, Some(false));
            let encoded = serde_json::to_value(result).unwrap();
            let mcp: serde_json::Value =
                serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
            assert_eq!(mcp, rest);
        }
    }
    PgAuth::new(pool, None).logout(&token).await.unwrap();
    assert!(client
        .call_tool(CallToolRequestParams::new("companies_list"))
        .await
        .is_err());
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = false)]
#[ignore = "requires isolated PostgreSQL databases"]
async fn upgrades_p02_without_changing_existing_identity(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(all.iter().take(2).cloned().collect()),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let actor = user(&pool, "existing@example.test", true).await;
    all.run(&pool).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT email FROM app_user WHERE id=$1")
            .bind(actor.parse::<i64>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap(),
        "existing@example.test"
    );
    assert!(PgCompany(pool).list(&actor).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn deletion_checks_owner_revision_and_preserves_audit(pool: PgPool) {
    let (a, token) = login(&pool).await;
    let b = user(&pool, "viewer@example.test", true).await;
    let store = PgCompany(pool.clone());
    let c = company::create(&store, &a, KEY, complete()).await.unwrap();
    assert_eq!(
        company::delete(&store, &b, &c.id, 1).await,
        Err(CompanyError::NotFound)
    );
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES ($1,$2,'viewer')")
        .bind(c.id.parse::<i64>().unwrap())
        .bind(b.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        company::delete(&store, &b, &c.id, 1).await,
        Err(CompanyError::Forbidden)
    );
    assert_eq!(
        company::delete(&store, &a, &c.id, 99).await,
        Err(CompanyError::Conflict)
    );
    let app = faktura_server::company::router(pool.clone(), vec!["https://app.test".into()], false);
    for (origin, expected) in [
        ("https://other.test", StatusCode::UNAUTHORIZED),
        ("https://app.test", StatusCode::NO_CONTENT),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/companies/{}", c.id))
                    .header("cookie", format!("faktura_session={token}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from("{\"revision\":1}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(company::list(&store, &a).await.unwrap().is_empty());
    assert!(matches!(
        company::context(&store, &a, &c.id).await,
        Err(CompanyError::NotFound)
    ));
    assert!(matches!(
        company::context(&store, &b, &c.id).await,
        Err(CompanyError::NotFound)
    ));
    assert_eq!(company::delete(&store, &a, &c.id, 1).await, Ok(()));
    assert!(matches!(
        company::update(&store, &a, &c.id, 2, 0, false, complete()).await,
        Err(CompanyError::NotFound)
    ));
    assert!(matches!(
        company::create(&store, &a, KEY, complete()).await,
        Err(CompanyError::NotFound)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM company_audit WHERE action='delete'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

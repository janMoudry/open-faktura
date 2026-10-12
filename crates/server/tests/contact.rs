use faktura_application::{
    auth::{self, AuthStore},
    company,
    contact::{self, Create, Error, Search, Update},
};
use faktura_domain::{
    company::CompanySettings,
    contact::{Address, ContactData, DeliveryAddress, Person},
};
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, contact::PgContact};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data(name: &str) -> ContactData {
    ContactData {
        name: name.into(),
        billing_address: Address {
            country: "CZ".into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn create(name: &str, key: &str) -> Create {
    Create {
        request_id: key.into(),
        data: data(name),
    }
}
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        pool.clone(),
        Some("synthetic-contact-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-contact-bootstrap-key-only-tests",
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
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let second = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        "00000000-0000-4000-8000-000000000002",
        CompanySettings::default(),
    )
    .await
    .unwrap();
    (actor.id, token, first.id, second.id)
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn contact_scope_roles_concurrency_archive_and_audit(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgContact(pool.clone());
    let (a, b) = tokio::join!(
        contact::create(&store, &actor, &company, create("Alpha", KEY)),
        contact::create(&store, &actor, &company, create("Alpha", KEY))
    );
    let first = a.unwrap();
    assert_eq!(first.id, b.unwrap().id);
    assert!(matches!(
        contact::create(&store, &actor, &company, create("Different", KEY)).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        contact::get(&store, &actor, &other, &first.id).await,
        Err(Error::NotFound)
    ));
    let outsider:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES ('outsider@example.test','no-login',true) RETURNING id").fetch_one(&pool).await.unwrap();
    let outsider_id = outsider.to_string();
    assert!(matches!(
        contact::list(&store, &outsider.to_string(), &company, Search::default()).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES ($1,$2,'viewer')")
        .bind(company.parse::<i64>().unwrap())
        .bind(outsider)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        contact::get(&store, &outsider.to_string(), &company, &first.id)
            .await
            .is_ok()
    );
    assert!(matches!(
        contact::create(&store, &outsider_id, &company, create("Viewer", KEY)).await,
        Err(Error::Forbidden)
    ));
    sqlx::query("UPDATE membership SET role='editor' WHERE user_id=$1")
        .bind(outsider)
        .execute(&pool)
        .await
        .unwrap();
    let command = || Update {
        revision: 1,
        archived: false,
        data: data("Edited"),
    };
    let (a, b) = tokio::join!(
        contact::update(&store, &actor, &company, &first.id, command()),
        contact::update(&store, &outsider_id, &company, &first.id, command())
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(Error::Conflict)) || matches!(b, Err(Error::Conflict)));
    let archived = contact::update(
        &store,
        &actor,
        &company,
        &first.id,
        Update {
            revision: 2,
            archived: true,
            data: data("Edited"),
        },
    )
    .await
    .unwrap();
    assert!(archived.archived);
    assert_eq!(
        contact::list(&store, &actor, &company, Search::default())
            .await
            .unwrap()
            .total,
        0
    );
    let page = contact::list(
        &PgContact(pool.clone()),
        &actor,
        &company,
        Search {
            archived: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.total, 1);
    let restored = contact::update(
        &store,
        &actor,
        &company,
        &first.id,
        Update {
            revision: 3,
            archived: false,
            data: data("Edited"),
        },
    )
    .await
    .unwrap();
    assert_eq!(restored.revision, 4);
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM contact_audit WHERE contact_id=$1")
        .bind(first.id.parse::<i64>().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit, 4);
    company::delete(&PgCompany(pool), &actor, &company, 1)
        .await
        .unwrap();
    assert!(matches!(
        contact::get(&store, &actor, &company, &first.id).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn validation_search_paging_and_snapshots(pool: PgPool) {
    let (actor, _, company, _) = setup(&pool).await;
    let store = PgContact(pool.clone());
    let mut full = data("  Žaneta Test  ");
    full.email = "MAIN@EXAMPLE.TEST".into();
    full.persons = vec![Person {
        name: "Jan".into(),
        email: "person@example.test".into(),
        phone: "123".into(),
    }];
    full.recipients = vec!["a@example.test".into(), "A@example.test".into()];
    full.delivery_addresses = vec![DeliveryAddress {
        id: "delivery-1".into(),
        name: "Warehouse".into(),
        address: Address {
            country: "sk".into(),
            city: "Bratislava".into(),
            ..Default::default()
        },
    }];
    full.default_delivery_id = Some("delivery-1".into());
    full.maturity_days = Some(30);
    let result = contact::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: KEY.into(),
            data: full,
        },
    )
    .await
    .unwrap();
    assert_eq!(result.data.name, "Žaneta Test");
    assert_eq!(result.data.recipients.len(), 1);
    assert_eq!(result.data.delivery_addresses[0].address.country, "SK");
    let mut invalid = result.data.clone();
    invalid.default_delivery_id = Some("missing".into());
    invalid.maturity_days = Some(366);
    assert!(matches!(
        contact::update(
            &store,
            &actor,
            &company,
            &result.id,
            Update {
                revision: 1,
                archived: false,
                data: invalid
            }
        )
        .await,
        Err(Error::Invalid(_))
    ));
    for (n, name) in [(2, "Alpha"), (3, "Beta")] {
        contact::create(
            &store,
            &actor,
            &company,
            create(name, &format!("00000000-0000-4000-8000-{n:012}")),
        )
        .await
        .unwrap();
    }
    let page = contact::list(
        &store,
        &actor,
        &company,
        Search {
            page_size: 2,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.total, 3);
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].data.name, "Alpha");
    assert_eq!(
        contact::list(
            &store,
            &actor,
            &company,
            Search {
                query: "MAIN@".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .items[0]
            .id,
        result.id
    );
    assert_eq!(
        contact::list(
            &store,
            &actor,
            &company,
            Search {
                query: "%".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    assert!(matches!(
        contact::list(
            &store,
            &actor,
            &company,
            Search {
                page: 0,
                ..Default::default()
            }
        )
        .await,
        Err(Error::Invalid(_))
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn rest_and_mcp_share_authorization_and_commands(pool: PgPool) {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (actor, token, company, other) = setup(&pool).await;
    let app = faktura_server::contact::router(
        pool.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    let path = format!("/api/companies/{company}/contacts");
    let body = serde_json::json!({"request_id":KEY,"data":data("REST")});
    for (cookie, origin, expected) in [
        ("", "https://app.example.test", StatusCode::UNAUTHORIZED),
        (
            token.as_str(),
            "https://evil.example.test",
            StatusCode::UNAUTHORIZED,
        ),
        (token.as_str(), "https://app.example.test", StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .method("POST")
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    for (cookie, profile, ico, expected) in [
        ("", company.as_str(), "00006947", StatusCode::UNAUTHORIZED),
        (
            token.as_str(),
            "999999999",
            "00006947",
            StatusCode::NOT_FOUND,
        ),
        (
            token.as_str(),
            company.as_str(),
            "00006948",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/companies/{profile}/contacts/ares/{ico}"))
                    .header("cookie", format!("faktura_session={cookie}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
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
    assert!(tools.iter().any(|t| t.name == "contact_create"));
    let call = |name: &str, args: serde_json::Value| {
        CallToolRequestParams::new(name.to_owned())
            .with_arguments(args.as_object().unwrap().clone())
    };
    assert!(tools.iter().any(|t| t.name == "contact_lookup_ares"));
    let invalid_lookup = client
        .call_tool(call(
            "contact_lookup_ares",
            serde_json::json!({"company_id":company,"ico":"00006948"}),
        ))
        .await
        .unwrap();
    assert_eq!(invalid_lookup.is_error, Some(true));
    let response=client.call_tool(call("contact_create",serde_json::json!({"company_id":company,"request_id":"00000000-0000-4000-8000-000000000004","data":data("MCP")}))).await.unwrap();
    assert_eq!(response.is_error, Some(false));
    let saved = contact::list(
        &PgContact(pool.clone()),
        &actor,
        &company,
        Search {
            query: "MCP".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .items
    .remove(0);
    let response=client.call_tool(call("contact_update",serde_json::json!({"company_id":company,"id":saved.id,"revision":1,"archived":true,"data":saved.data}))).await.unwrap();
    assert_eq!(response.is_error, Some(false));
    let response = client
        .call_tool(call(
            "contact_get",
            serde_json::json!({"company_id":other,"id":saved.id}),
        ))
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(true));
    PgAuth::new(pool, None).logout(&token).await.unwrap();
    assert!(client
        .call_tool(call(
            "contacts_list",
            serde_json::json!({"company_id":company})
        ))
        .await
        .is_err());
    client.cancel().await.unwrap();
    task.await.unwrap();
}

use faktura_application::{
    auth::{self, AuthStore},
    catalog::{self, Create, Error, Search, Update},
    company,
};
use faktura_domain::{catalog::CatalogData, company::CompanySettings};
use faktura_infrastructure::{auth::PgAuth, catalog::PgCatalog, company::PgCompany};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data(name: &str) -> CatalogData {
    CatalogData {
        name: name.into(),
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
async fn catalog_scope_roles_concurrency_archive_and_audit(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgCatalog(pool.clone());
    let (a, b) = tokio::join!(
        catalog::create(&store, &actor, &company, create("Alpha", KEY)),
        catalog::create(&store, &actor, &company, create("Alpha", KEY))
    );
    let first = a.unwrap();
    assert_eq!(first.id, b.unwrap().id);
    assert!(matches!(
        catalog::create(&store, &actor, &company, create("Different", KEY)).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        catalog::get(&store, &actor, &other, &first.id).await,
        Err(Error::NotFound)
    ));
    let outsider:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES ('outsider@example.test','no-login',true) RETURNING id").fetch_one(&pool).await.unwrap();
    let outsider_id = outsider.to_string();
    assert!(matches!(
        catalog::list(&store, &outsider.to_string(), &company, Search::default()).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES ($1,$2,'viewer')")
        .bind(company.parse::<i64>().unwrap())
        .bind(outsider)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        catalog::get(&store, &outsider.to_string(), &company, &first.id)
            .await
            .is_ok()
    );
    assert!(matches!(
        catalog::create(&store, &outsider_id, &company, create("Viewer", KEY)).await,
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
        catalog::update(&store, &actor, &company, &first.id, command()),
        catalog::update(&store, &outsider_id, &company, &first.id, command())
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(Error::Conflict)) || matches!(b, Err(Error::Conflict)));
    let archived = catalog::update(
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
        catalog::list(&store, &actor, &company, Search::default())
            .await
            .unwrap()
            .total,
        0
    );
    let page = catalog::list(
        &PgCatalog(pool.clone()),
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
    let restored = catalog::update(
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
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM catalog_audit WHERE catalog_id=$1")
        .bind(first.id.parse::<i64>().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audit, 4);
    company::delete(&PgCompany(pool), &actor, &company, 1)
        .await
        .unwrap();
    assert!(matches!(
        catalog::get(&store, &actor, &company, &first.id).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn validation_search_paging_and_snapshots(pool: PgPool) {
    let (actor, _, company, _) = setup(&pool).await;
    let store = PgCatalog(pool.clone());
    let mut full = data("  Žaneta Test  ");
    full.code = "SKU-01".into();
    full.barcode = "12345678".into();
    full.unit_price = "121.0000".into();
    full.price_mode = faktura_domain::invoice::PriceMode::Gross;
    full.vat_percent = "21".into();
    let result = catalog::create(
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
    assert_eq!(result.data.unit_price, "121");
    let mut invalid = result.data.clone();
    invalid.quantity = "0".into();
    invalid.unit_price = "1e2".into();
    assert!(matches!(
        catalog::update(
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
    let options = || catalog::LineOptions {
        price_mode: faktura_domain::invoice::PriceMode::Net,
        tax_mode: faktura_domain::invoice::TaxMode::VatPayer,
    };
    let snapshot = catalog::line(&store, &actor, &company, &result.id, options())
        .await
        .unwrap();
    assert_eq!(snapshot.unit_price, "100");
    let mut changed = result.data.clone();
    changed.unit_price = "242".into();
    catalog::update(
        &store,
        &actor,
        &company,
        &result.id,
        Update {
            revision: 1,
            archived: false,
            data: changed.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(snapshot.unit_price, "100");
    assert_eq!(
        catalog::line(&store, &actor, &company, &result.id, options())
            .await
            .unwrap()
            .unit_price,
        "200"
    );
    catalog::update(
        &store,
        &actor,
        &company,
        &result.id,
        Update {
            revision: 2,
            archived: true,
            data: changed.clone(),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        catalog::line(&store, &actor, &company, &result.id, options()).await,
        Err(Error::Invalid(_))
    ));
    catalog::update(
        &store,
        &actor,
        &company,
        &result.id,
        Update {
            revision: 3,
            archived: false,
            data: changed,
        },
    )
    .await
    .unwrap();
    for (n, name) in [(2, "Alpha"), (3, "Beta")] {
        catalog::create(
            &store,
            &actor,
            &company,
            create(name, &format!("00000000-0000-4000-8000-{n:012}")),
        )
        .await
        .unwrap();
    }
    let page = catalog::list(
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
        catalog::list(
            &store,
            &actor,
            &company,
            Search {
                query: "sku-01".into(),
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
        catalog::list(
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
        catalog::list(
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
    let app = faktura_server::catalog::router(
        pool.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    let path = format!("/api/companies/{company}/catalog");
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
    assert!(tools.iter().any(|t| t.name == "catalog_create"));
    let call = |name: &str, args: serde_json::Value| {
        CallToolRequestParams::new(name.to_owned())
            .with_arguments(args.as_object().unwrap().clone())
    };
    let response=client.call_tool(call("catalog_create",serde_json::json!({"company_id":company,"request_id":"00000000-0000-4000-8000-000000000004","data":data("MCP")}))).await.unwrap();
    assert_eq!(response.is_error, Some(false));
    let saved = catalog::list(
        &PgCatalog(pool.clone()),
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
    let rest_line = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "{path}/{}/line?price_mode=net&tax_mode=non_payer",
                    saved.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rest_line.status(), StatusCode::OK);
    let rest_value: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(rest_line.into_body(), 65536)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(rest_value["description"], "MCP");
    let mcp_line = client.call_tool(call("catalog_line", serde_json::json!({"company_id":company,"id":saved.id,"price_mode":"net","tax_mode":"non_payer"}))).await.unwrap();
    assert_eq!(mcp_line.is_error, Some(false));
    let mcp_value: serde_json::Value =
        serde_json::from_str(&mcp_line.content[0].as_text().unwrap().text).unwrap();
    for field in [
        "description",
        "quantity",
        "unit",
        "unit_price",
        "vat_percent",
        "discount_percent",
    ] {
        assert_eq!(rest_value[field], mcp_value[field]);
    }
    let response=client.call_tool(call("catalog_update",serde_json::json!({"company_id":company,"id":saved.id,"revision":1,"archived":true,"data":saved.data}))).await.unwrap();
    assert_eq!(response.is_error, Some(false));
    let response = client
        .call_tool(call(
            "catalog_get",
            serde_json::json!({"company_id":other,"id":saved.id}),
        ))
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(true));
    PgAuth::new(pool, None).logout(&token).await.unwrap();
    assert!(client
        .call_tool(call(
            "catalogs_list",
            serde_json::json!({"company_id":company})
        ))
        .await
        .is_err());
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = false)]
#[ignore = "requires isolated PostgreSQL"]
async fn catalog_upgrade_preserves_previous_data(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|m| m.version < 202610110011)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (actor, _, company, _) = setup(&pool).await;
    let contact = faktura_application::contact::create(
        &faktura_infrastructure::contact::PgContact(pool.clone()),
        &actor,
        &company,
        faktura_application::contact::Create {
            request_id: KEY.into(),
            data: faktura_domain::contact::ContactData {
                name: "Existing customer".into(),
                billing_address: faktura_domain::contact::Address {
                    country: "CZ".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        },
    )
    .await
    .unwrap();
    all.run(&pool).await.unwrap();
    let after = faktura_application::contact::get(
        &faktura_infrastructure::contact::PgContact(pool.clone()),
        &actor,
        &company,
        &contact.id,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::to_value(contact).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    assert!(catalog::create(
        &PgCatalog(pool),
        &actor,
        &company,
        create("New catalog item", KEY)
    )
    .await
    .is_ok());
}

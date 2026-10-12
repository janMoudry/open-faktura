use faktura_application::{
    auth, company,
    sales::{self, Convert, Create, Data, Error, Kind, Search, Update},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, draft::PgDraft, sales::PgSales};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> Data {
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.tax_on = None;
    invoice.calculation.lines[0].unit_price = "1200".into();
    invoice.calculation.lines[0].quantity = "2".into();
    Data {
        name: "Test quotation".into(),
        maturity_days: 14,
        status: "created".into(),
        invoice,
    }
}
fn create(kind: Kind, key: &str) -> Create {
    Create {
        request_id: key.into(),
        kind,
        data: data(),
    }
}
fn conversion(revision: i64, key: &str) -> Convert {
    Convert {
        request_id: key.into(),
        revision,
        issued_on: "2026-10-11".parse().unwrap(),
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
#[ignore = "isolated PostgreSQL"]
async fn quotation_numbering_conversion_replay_scope_and_issue(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let s = PgSales(pool.clone());
    let (x, y) = tokio::join!(
        sales::create(&s, &a, &c, create(Kind::Quote, KEY)),
        sales::create(&s, &a, &c, create(Kind::Quote, KEY))
    );
    let q = x.unwrap();
    assert_eq!(q.id, y.unwrap().id);
    assert_eq!(q.number.as_deref(), Some("CN20260001"));
    assert!(matches!(
        sales::get(&s, &a, &other, &q.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        sales::create(&s, &a, &c, create(Kind::Template, KEY)).await,
        Err(Error::Conflict)
    ));
    let (x, y) = tokio::join!(
        sales::convert(&s, &a, &c, &q.id, conversion(1, KEY)),
        sales::convert(
            &s,
            &a,
            &c,
            &q.id,
            conversion(1, "00000000-0000-4000-8000-000000000002")
        )
    );
    let d = x.unwrap();
    assert_eq!(d.id, y.unwrap().id);
    assert!(d.number.is_none());
    assert_eq!(d.totals.as_ref().unwrap().payable, "2400.00");
    assert_eq!(d.data.due_on.unwrap().to_string(), "2026-10-25");
    assert_eq!(
        d.totals.as_ref().unwrap().payable,
        q.totals.as_ref().unwrap().payable
    );
    assert_eq!(
        sales::convert(&s, &a, &c, &q.id, conversion(1, KEY))
            .await
            .unwrap()
            .id,
        d.id
    );
    assert!(matches!(
        sales::convert(&s, &a, &other, &q.id, conversion(1, KEY)).await,
        Err(Error::NotFound)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let issued = faktura_application::draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &d.id,
        faktura_application::draft::Issue {
            request_id: KEY.into(),
            revision: 1,
        },
    )
    .await
    .unwrap();
    assert!(issued.number.is_some());
    assert_eq!(
        sales::get(&s, &a, &c, &q.id).await.unwrap().data.status,
        "invoiced"
    );
    assert_eq!(
        sales::list(
            &s,
            &a,
            &c,
            Search {
                status: "invoiced".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    let q2 = sales::create(
        &s,
        &a,
        &c,
        create(Kind::Quote, "00000000-0000-4000-8000-000000000003"),
    )
    .await
    .unwrap();
    assert_eq!(q2.number.as_deref(), Some("CN20260002"));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn template_revision_archive_dates_snapshot_and_roles(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let s = PgSales(pool.clone());
    let t = sales::create(&s, &a, &c, create(Kind::Template, KEY))
        .await
        .unwrap();
    assert!(t.number.is_none());
    let mut changed = t.data.clone();
    changed.name = "Updated template".into();
    let command = || Update {
        revision: 1,
        archived: false,
        data: changed.clone(),
    };
    let (x, y) = tokio::join!(
        sales::update(&s, &a, &c, &t.id, command()),
        sales::update(&s, &a, &c, &t.id, command())
    );
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    let d = sales::convert(&s, &a, &c, &t.id, conversion(2, KEY))
        .await
        .unwrap();
    let d2 = sales::convert(
        &s,
        &a,
        &c,
        &t.id,
        conversion(2, "00000000-0000-4000-8000-000000000002"),
    )
    .await
    .unwrap();
    assert_ne!(d.id, d2.id);
    changed.invoice.calculation.lines[0].unit_price = "999".into();
    sales::update(
        &s,
        &a,
        &c,
        &t.id,
        Update {
            revision: 2,
            archived: true,
            data: changed,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        sales::convert(
            &s,
            &a,
            &c,
            &t.id,
            conversion(3, "00000000-0000-4000-8000-000000000003")
        )
        .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        sales::list(
            &s,
            &a,
            &c,
            Search {
                kind: Kind::Template,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    assert_eq!(
        sales::list(
            &s,
            &a,
            &c,
            Search {
                kind: Kind::Template,
                archived: true,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        faktura_application::draft::get(&PgDraft(pool.clone()), &a, &c, &d.id)
            .await
            .unwrap()
            .data
            .calculation
            .lines[0]
            .unit_price,
        d.data.calculation.lines[0].unit_price
    );
    let outsider:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES('viewer@example.test','no-login',true) RETURNING id").fetch_one(&pool).await.unwrap();
    let actor = outsider.to_string();
    assert!(matches!(
        sales::list(&s, &actor, &c, Search::default()).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(c.parse::<i64>().unwrap())
        .bind(outsider)
        .execute(&pool)
        .await
        .unwrap();
    assert!(sales::get(&s, &actor, &c, &t.id).await.is_ok());
    assert!(matches!(
        sales::create(&s, &actor, &c, create(Kind::Quote, KEY)).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        sales::convert(&s, &actor, &c, &t.id, conversion(3, KEY)).await,
        Err(Error::Forbidden)
    ));
    assert!(sales::list(&s, &a, &other, Search::default())
        .await
        .unwrap()
        .items
        .is_empty());
    let mut bad = create(Kind::Quote, "00000000-0000-4000-8000-000000000004");
    bad.data.invoice.customer = None;
    assert!(matches!(
        sales::create(&s, &a, &c, bad).await,
        Err(Error::Invalid(_))
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_conversion_authorization_and_parity(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (a, token, c, other) = setup(&pool).await;
    let app =
        faktura_server::sales::router(pool.clone(), vec!["https://app.example.test".into()], false);
    let path = format!("/api/companies/{c}/sales");
    let body = serde_json::to_value(create(Kind::Quote, KEY)).unwrap();
    for (cookie, origin, expected) in [
        ("", "https://app.example.test", StatusCode::UNAUTHORIZED),
        (
            token.as_str(),
            "https://evil.example.test",
            StatusCode::UNAUTHORIZED,
        ),
        (token.as_str(), "https://app.example.test", StatusCode::OK),
    ] {
        let r = app
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
        assert_eq!(r.status(), expected);
        assert_eq!(r.headers()["cache-control"], "no-store");
    }
    let q = sales::list(&PgSales(pool.clone()), &a, &c, Search::default())
        .await
        .unwrap()
        .items
        .remove(0);
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
    assert!(client
        .list_all_tools()
        .await
        .unwrap()
        .iter()
        .any(|t| t.name == "sales_convert"));
    let call = |company: &str| {
        CallToolRequestParams::new("sales_convert").with_arguments(serde_json::json!({"company_id":company,"id":q.id,"request_id":KEY,"revision":1,"issued_on":"2026-10-11"}).as_object().unwrap().clone())
    };
    assert_eq!(
        client.call_tool(call(&other)).await.unwrap().is_error,
        Some(true)
    );
    let response = client.call_tool(call(&c)).await.unwrap();
    assert_eq!(response.is_error, Some(false));
    let mcp: serde_json::Value =
        serde_json::from_str(&response.content[0].as_text().unwrap().text).unwrap();
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("{path}/{}/convert", q.id))
                .method("POST")
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "https://app.example.test")
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&conversion(1, KEY)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let rest: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(rest, mcp);
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade_preserves_existing_invoices(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|m| m.version < 202610110012)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let d = faktura_application::draft::create(
        &PgDraft(pool.clone()),
        &a,
        &c,
        faktura_application::draft::Create {
            request_id: KEY.into(),
            data: data().invoice,
        },
    )
    .await
    .unwrap();
    all.run(&pool).await.unwrap();
    let preserved = faktura_application::draft::get(&PgDraft(pool.clone()), &a, &c, &d.id)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(d).unwrap(),
        serde_json::to_value(preserved).unwrap()
    );
    assert!(
        sales::create(&PgSales(pool), &a, &c, create(Kind::Template, KEY))
            .await
            .is_ok()
    );
}

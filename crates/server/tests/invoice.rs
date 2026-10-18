use faktura_application::{
    auth, company,
    draft::{self, Create, Error, Search, Update},
};
use faktura_domain::{company::CompanySettings, invoice::DraftData};
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, draft::PgDraft};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> DraftData {
    serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap()
}
fn create(key: &str) -> Create {
    Create {
        request_id: key.into(),
        data: data(),
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
#[ignore = "requires isolated PostgreSQL"]
async fn persistence_replay_revision_and_roles(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    let (a, b) = tokio::join!(
        draft::create(&store, &actor, &company, create(KEY)),
        draft::create(&store, &actor, &company, create(KEY))
    );
    let a = a.unwrap();
    assert_eq!(a.id, b.unwrap().id);
    assert_eq!(a.totals.unwrap().payable, "72.59");
    let fresh = PgDraft(pool.clone());
    assert_eq!(
        draft::get(&fresh, &actor, &company, &a.id)
            .await
            .unwrap()
            .data
            .customer
            .unwrap()
            .name,
        "Synthetic customer"
    );
    let mut changed = create(KEY);
    changed.data.note = "different".into();
    assert!(matches!(
        draft::create(&store, &actor, &company, changed).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        draft::get(&store, &actor, &other, &a.id).await,
        Err(Error::NotFound)
    ));
    let outsider:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES ('other@example.test','no-login',true) RETURNING id").fetch_one(&pool).await.unwrap();
    let outsider = outsider.to_string();
    assert!(matches!(
        draft::list(&store, &outsider, &company, Search::default()).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES ($1::text::bigint,$2::text::bigint,'viewer')").bind(&company).bind(&outsider).execute(&pool).await.unwrap();
    assert!(draft::get(&store, &outsider, &company, &a.id).await.is_ok());
    assert!(matches!(
        draft::create(&store, &outsider, &company, create(KEY)).await,
        Err(Error::Forbidden)
    ));
    sqlx::query("UPDATE membership SET role='editor' WHERE user_id=$1::text::bigint")
        .bind(&outsider)
        .execute(&pool)
        .await
        .unwrap();
    let command = || Update {
        revision: 1,
        data: data(),
    };
    let (x, y) = tokio::join!(
        draft::update(&store, &actor, &company, &a.id, command()),
        draft::update(&store, &outsider, &company, &a.id, command())
    );
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft_audit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(audits, 2);
    let mut incomplete = data();
    incomplete.calculation.lines[0].unit_price = "".into();
    incomplete.customer = None;
    incomplete.issued_on = None;
    let updated = draft::update(
        &store,
        &actor,
        &company,
        &a.id,
        Update {
            revision: 2,
            data: incomplete,
        },
    )
    .await
    .unwrap();
    assert!(updated.totals.is_none());
    let restored = draft::get(&store, &actor, &company, &a.id).await.unwrap();
    assert_eq!(restored.data.calculation.lines[0].unit_price, "");
    company::delete(&PgCompany(pool.clone()), &actor, &company, 1)
        .await
        .unwrap();
    assert!(matches!(
        draft::get(&store, &actor, &company, &a.id).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn contact_snapshot_filter_paging_and_validation(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    let mut contact = faktura_domain::contact::ContactData {
        name: "Original customer".into(),
        ..Default::default()
    };
    contact.billing_address.country = "CZ".into();
    let c = faktura_application::contact::create(
        &faktura_infrastructure::contact::PgContact(pool.clone()),
        &actor,
        &company,
        faktura_application::contact::Create {
            request_id: KEY.into(),
            data: contact,
        },
    )
    .await
    .unwrap();
    let mut command = create(KEY);
    command.data.contact_id = Some(c.id.clone());
    let a = draft::create(&store, &actor, &company, command)
        .await
        .unwrap();
    let mut foreign = create(KEY);
    foreign.data.contact_id = Some(c.id.clone());
    assert!(matches!(
        draft::create(&store, &actor, &other, foreign).await,
        Err(Error::NotFound)
    ));
    sqlx::query(
        "UPDATE contact SET data=jsonb_set(data,'{name}','\"Changed\"') WHERE id=$1::text::bigint",
    )
    .bind(&c.id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        draft::get(&store, &actor, &company, &a.id)
            .await
            .unwrap()
            .data
            .customer
            .unwrap()
            .name,
        "Synthetic customer"
    );
    let b = draft::create(
        &store,
        &actor,
        &company,
        create("00000000-0000-4000-8000-000000000002"),
    )
    .await
    .unwrap();
    assert_ne!(a.id, b.id);
    // Inclusive date and decimal amount bounds, applied before pagination.
    for (search, expected) in [
        (
            Search {
                date_from: Some("2026-10-11".into()),
                date_to: Some("2026-10-11".into()),
                amount_min: Some("72.59".into()),
                amount_max: Some("72.59".into()),
                page_size: 1,
                sort: "amount_desc".into(),
                ..Default::default()
            },
            2,
        ),
        (
            Search {
                amount_min: Some("72.60".into()),
                ..Default::default()
            },
            0,
        ),
        (
            Search {
                date_to: Some("2026-10-10".into()),
                ..Default::default()
            },
            0,
        ),
        (
            Search {
                date_field: "due_on".into(),
                date_from: Some("2027-01-01".into()),
                ..Default::default()
            },
            0,
        ),
    ] {
        let result = draft::list(&store, &actor, &company, search.clone())
            .await
            .unwrap();
        assert_eq!(result.total, expected);
        assert!(result.items.len() <= search.page_size as usize);
        assert_eq!(
            draft::list(&store, &actor, &other, search)
                .await
                .unwrap()
                .total,
            0
        );
    }
    for search in [
        Search {
            date_from: Some("2026-02-30".into()),
            ..Default::default()
        },
        Search {
            date_from: Some("2026-10-12".into()),
            date_to: Some("2026-10-11".into()),
            ..Default::default()
        },
        Search {
            amount_min: Some("NaN".into()),
            ..Default::default()
        },
        Search {
            amount_min: Some("73".into()),
            amount_max: Some("72".into()),
            ..Default::default()
        },
        Search {
            date_field: "unsafe".into(),
            ..Default::default()
        },
    ] {
        assert!(matches!(
            draft::list(&store, &actor, &company, search).await,
            Err(Error::Invalid(_))
        ));
    }
    let page = draft::list(
        &store,
        &actor,
        &company,
        Search {
            page_size: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.items.len(), 1);
    let page = draft::list(
        &store,
        &actor,
        &company,
        Search {
            customer_id: Some(c.id),
            sort: "customer_asc".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(
        draft::list(
            &store,
            &actor,
            &company,
            Search {
                query: "ORDER-1".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        2
    );
    assert_eq!(
        draft::list(
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
    let mut bad = create("00000000-0000-4000-8000-000000000003");
    bad.data.calculation.lines[0].unit_price = "NaN".into();
    assert!(matches!(
        draft::create(&store, &actor, &company, bad).await,
        Err(Error::Invalid(_))
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn rest_and_mcp_drafts(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (_, token, company, other) = setup(&pool).await;
    let app = faktura_server::invoice::router(
        pool.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    let body = serde_json::json!({"request_id":KEY,"data":issuable()});
    let mut saved = None;
    for (cookie, origin, status) in [
        ("", "https://app.example.test", 401),
        (token.as_str(), "https://evil.example.test", 401),
        (token.as_str(), "https://app.example.test", 200),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/companies/{company}/invoices"))
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        if status == 200 {
            saved = Some(
                serde_json::from_slice::<serde_json::Value>(
                    &to_bytes(response.into_body(), 65536).await.unwrap(),
                )
                .unwrap(),
            );
        }
    }
    let saved = saved.unwrap();
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
    // Identical filter and sorting arguments in REST and MCP.
    let response = app.clone().oneshot(Request::builder()
        .uri(format!("/api/companies/{company}/invoices?date_field=issued_on&date_from=2026-10-11&date_to=2026-10-11&amount_min=0&amount_max=100&sort=amount_desc&page_size=10"))
        .header("cookie", format!("faktura_session={token}"))
        .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), 200);
    let rest: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    let args = serde_json::json!({"company_id":company,"date_field":"issued_on","date_from":"2026-10-11","date_to":"2026-10-11","amount_min":"0","amount_max":"100","sort":"amount_desc","page_size":10});
    let result = client
        .call_tool(
            CallToolRequestParams::new("invoices_list")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let encoded = serde_json::to_value(result).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rest, mcp);
    assert_eq!(rest["total"], 1);
    for (profile, failed) in [(&company, false), (&other, true)] {
        let args = serde_json::json!({"company_id":profile,"id":saved["id"]});
        let result = client
            .call_tool(
                CallToolRequestParams::new("invoice_get")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(failed));
        if !failed {
            let encoded = serde_json::to_value(result).unwrap();
            let parsed: serde_json::Value =
                serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
            assert_eq!(parsed, saved);
        }
    }
    let args =
        serde_json::json!({"company_id":company,"id":saved["id"],"revision":1,"data":issuable()});
    let result = client
        .call_tool(
            CallToolRequestParams::new("invoice_update_draft")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let args =
        serde_json::json!({"company_id":company,"id":saved["id"],"revision":2,"request_id":KEY});
    let issued = client
        .call_tool(
            CallToolRequestParams::new("invoice_issue")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(issued.is_error, Some(false));
    let encoded = serde_json::to_value(issued).unwrap();
    let issued: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    let uri = format!(
        "/api/companies/{company}/invoices/{}/issue",
        saved["id"].as_str().unwrap()
    );
    for (origin, expected) in [
        ("https://evil.example.test", 401),
        ("https://app.example.test", 200),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .header("cookie", format!("faktura_session={token}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"revision":2,"request_id":KEY}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), expected);
        if expected == 200 {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(
                    &to_bytes(r.into_body(), 65536).await.unwrap()
                )
                .unwrap(),
                issued
            );
        }
    }
    let pdf_args = serde_json::json!({"company_id":company,"id":saved["id"]});
    let pdf = client
        .call_tool(
            CallToolRequestParams::new("invoice_pdf")
                .with_arguments(pdf_args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(pdf.is_error, Some(false));
    let encoded = serde_json::to_value(pdf).unwrap();
    let pdf: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(pdf["base64"].as_str().unwrap())
        .unwrap();
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{company}/invoices/{}/pdf",
                    saved["id"].as_str().unwrap()
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.headers()["content-type"], "application/pdf");
    assert_eq!(
        to_bytes(r.into_body(), 10000000).await.unwrap().as_ref(),
        bytes
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

fn issuable() -> DraftData {
    let mut d = data();
    d.supplier.registration_number = "12345678".into();
    d.supplier.vat_number = "CZ12345678".into();
    d.payment.method = "cash".into();
    d.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    d.calculation.lines[0].vat_percent = "0".into();
    d.tax_on = None;
    d.calculation.lines[0].description = "Příliš žluťoučký kůň – návrh řešení".into();
    d
}
fn issue_command(key: &str, revision: i64) -> draft::Issue {
    draft::Issue {
        request_id: key.into(),
        revision,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn issue_concurrency_replay_immutability_pdf(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    let a = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: KEY.into(),
            data: issuable(),
        },
    )
    .await
    .unwrap();
    let key2 = "00000000-0000-4000-8000-000000000002";
    let b = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: key2.into(),
            data: issuable(),
        },
    )
    .await
    .unwrap();
    let (x, y) = tokio::join!(
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 1)),
        draft::issue(&store, &actor, &company, &b.id, issue_command(key2, 1))
    );
    let x = x.unwrap();
    let y = y.unwrap();
    let mut numbers = vec![x.number.clone().unwrap(), y.number.unwrap()];
    numbers.sort();
    assert_eq!(numbers, ["FV2026000001", "FV2026000002"]);
    assert_eq!(x.revision, 2);
    assert_eq!(
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 1))
            .await
            .unwrap()
            .number,
        x.number
    );
    assert!(matches!(
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 2)).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        draft::issue(&store, &actor, &company, &b.id, issue_command(KEY, 1)).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        draft::issue(&store, &actor, &other, &a.id, issue_command(KEY, 1)).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        draft::update(
            &store,
            &actor,
            &company,
            &a.id,
            Update {
                revision: 2,
                data: issuable()
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    assert!(
        sqlx::query("UPDATE invoice_draft SET data='{}' WHERE id=$1::text::bigint")
            .bind(&a.id)
            .execute(&pool)
            .await
            .is_err()
    );
    let pdf = draft::pdf(&store, &actor, &company, &a.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{name}','\"changed\"') WHERE id=$1::text::bigint").bind(&company).execute(&pool).await.unwrap();
    assert_eq!(
        draft::pdf(&store, &actor, &company, &a.id).await.unwrap(),
        pdf
    );
    assert!(matches!(
        draft::pdf(&store, &actor, &other, &a.id).await,
        Err(Error::NotFound)
    ));
    let profile = company::context(&PgCompany(pool.clone()), &actor, &company)
        .await
        .unwrap();
    assert_eq!(profile.settings.next_number, 3);
    assert_eq!(
        draft::list(
            &store,
            &actor,
            &company,
            Search {
                status: "draft".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    assert_eq!(
        draft::list(
            &store,
            &actor,
            &company,
            Search {
                query: x.number.unwrap(),
                status: "issued".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    std::fs::create_dir_all("../../tmp/pdfs").unwrap();
    std::fs::write("../../tmp/pdfs/issued-cs.pdf", pdf).unwrap();
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn issue_validation_races_and_permissions(pool: PgPool) {
    let (actor, _, company, _) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    let mut d = issuable();
    d.customer = None;
    let a = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: KEY.into(),
            data: d,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 1)).await,
        Err(Error::Invalid(_))
    ));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_number_series")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    draft::update(
        &store,
        &actor,
        &company,
        &a.id,
        Update {
            revision: 1,
            data: issuable(),
        },
    )
    .await
    .unwrap();
    let (x, y) = tokio::join!(
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 2)),
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 2))
    );
    assert_eq!(x.unwrap().number, y.unwrap().number);
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES('viewer@issue.test','no-login',true) RETURNING id").fetch_one(&pool).await.unwrap();
    let viewer = viewer.to_string();
    assert!(matches!(
        draft::get(&store, &viewer, &company, &a.id).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1::text::bigint,$2::text::bigint,'viewer')").bind(&company).bind(&viewer).execute(&pool).await.unwrap();
    assert!(draft::pdf(&store, &viewer, &company, &a.id).await.is_ok());
    assert!(matches!(
        draft::issue(&store, &viewer, &company, &a.id, issue_command(KEY, 2)).await,
        Err(Error::Forbidden)
    ));
    let audits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM invoice_draft_audit WHERE action='issue'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audits, 1);
    // Rewinding the UI counter cannot reuse a number, even across settings changes.
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{next_number}','1') WHERE id=$1::text::bigint").bind(&company).execute(&pool).await.unwrap();
    let key2 = "00000000-0000-4000-8000-000000000002";
    let mut long = issuable();
    long.language = "en".into();
    long.calculation.lines = (0..80)
        .map(|i| {
            let mut l = long.calculation.lines[0].clone();
            l.id = i.to_string();
            l.description = format!(
                "Item {i}: {}",
                "Long description with česká diakritika. ".repeat(12)
            );
            l
        })
        .collect();
    let b = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: key2.into(),
            data: long,
        },
    )
    .await
    .unwrap();
    let b = draft::issue(&store, &actor, &company, &b.id, issue_command(key2, 1))
        .await
        .unwrap();
    assert_eq!(b.number.as_deref(), Some("FV2026000002"));
    let pdf = draft::pdf(&store, &actor, &company, &b.id).await.unwrap();
    std::fs::create_dir_all("../../tmp/pdfs").unwrap();
    std::fs::write("../../tmp/pdfs/issued-en-long.pdf", pdf).unwrap();
}

#[sqlx::test(migrations = false)]
#[ignore = "requires isolated PostgreSQL"]
async fn upgrade_p06_preserves_existing_draft(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(all.iter().take(7).cloned().collect()),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (actor, _, company, _) = setup(&pool).await;
    let data = serde_json::to_value(issuable()).unwrap();
    let id:i64=sqlx::query_scalar("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data) VALUES($1::text::bigint,$2::text::bigint,$3::text::uuid,$4,$4) RETURNING id").bind(&company).bind(&actor).bind(KEY).bind(&data).fetch_one(&pool).await.unwrap();
    all.run(&pool).await.unwrap();
    let value = draft::get(&PgDraft(pool), &actor, &company, &id.to_string())
        .await
        .unwrap();
    assert!(value.number.is_none());
    assert!(value.issued_at.is_none());
    assert_eq!(value.revision, 1);
    assert_eq!(serde_json::to_value(value.data).unwrap(), data);
}

#[test]
fn issue_required_fields_and_dates() {
    use faktura_domain::invoice::{validate_issue, TaxMode};
    assert!(validate_issue(&issuable()).is_ok());
    let mut d = issuable();
    d.calculation.lines.clear();
    assert!(validate_issue(&d).unwrap_err().contains(&"lines".into()));
    let mut d = issuable();
    d.calculation.lines[0].description = " ".into();
    d.calculation.lines[0].quantity = "0".into();
    d.due_on = Some("2025-01-01".parse().unwrap());
    d.payment.method = "bank_transfer".into();
    let errors = validate_issue(&d).unwrap_err();
    for key in [
        "lines.0.description",
        "lines.0.quantity",
        "due_on",
        "payment.bank_account",
    ] {
        assert!(errors.contains(&key.to_owned()));
    }
    let mut d = issuable();
    d.calculation.tax_mode = TaxMode::VatPayer;
    d.supplier.vat_number.clear();
    let errors = validate_issue(&d).unwrap_err();
    assert!(errors.contains(&"tax_on".into()));
    assert!(errors.contains(&"supplier.vat_number".into()));
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn numbering_year_tax_mode_exhaustion_and_update_race(pool: PgPool) {
    let (actor, _, company, _) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    sqlx::query("INSERT INTO company_tax_profile(company_id,revision,mode,effective_from) VALUES($1::text::bigint,1,'non_payer','2027-01-01')").bind(&company).execute(&pool).await.unwrap();
    let future_key = "00000000-0000-4000-8000-000000000099";
    let future = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: future_key.into(),
            data: issuable(),
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(draft::issue(&store,&actor,&company,&future.id,issue_command(future_key,1)).await,Err(Error::Invalid(v)) if v.contains(&"tax_mode".into()))
    );
    sqlx::query("DELETE FROM company_tax_profile WHERE company_id=$1::text::bigint")
        .bind(&company)
        .execute(&pool)
        .await
        .unwrap();
    let mut wrong = issuable();
    wrong.issued_on = Some("2025-12-31".parse().unwrap());
    let a = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: KEY.into(),
            data: wrong,
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(draft::issue(&store,&actor,&company,&a.id,issue_command(KEY,1)).await,Err(Error::Invalid(v)) if v.contains(&"number_year".into()))
    );
    let mut vat = issuable();
    vat.calculation.tax_mode = faktura_domain::invoice::TaxMode::VatPayer;
    vat.tax_on = vat.issued_on;
    draft::update(
        &store,
        &actor,
        &company,
        &a.id,
        Update {
            revision: 1,
            data: vat,
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(draft::issue(&store,&actor,&company,&a.id,issue_command(KEY,2)).await,Err(Error::Invalid(v)) if v.contains(&"tax_mode".into()))
    );
    draft::update(
        &store,
        &actor,
        &company,
        &a.id,
        Update {
            revision: 2,
            data: issuable(),
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{next_number}','999999') WHERE id=$1::text::bigint").bind(&company).execute(&pool).await.unwrap();
    assert_eq!(
        draft::issue(&store, &actor, &company, &a.id, issue_command(KEY, 3))
            .await
            .unwrap()
            .number
            .as_deref(),
        Some("FV2026999999")
    );
    let key2 = "00000000-0000-4000-8000-000000000002";
    let b = draft::create(
        &store,
        &actor,
        &company,
        Create {
            request_id: key2.into(),
            data: issuable(),
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(draft::issue(&store,&actor,&company,&b.id,issue_command(key2,1)).await,Err(Error::Invalid(v)) if v.contains(&"next_number".into()))
    );
    sqlx::query("UPDATE company SET settings=jsonb_set(jsonb_set(settings,'{number_year}','2027'),'{next_number}','1') WHERE id=$1::text::bigint").bind(&company).execute(&pool).await.unwrap();
    let mut next = issuable();
    next.issued_on = Some("2027-01-01".parse().unwrap());
    next.due_on = Some("2027-01-15".parse().unwrap());
    draft::update(
        &store,
        &actor,
        &company,
        &b.id,
        Update {
            revision: 1,
            data: next.clone(),
        },
    )
    .await
    .unwrap();
    let (issue, edit) = tokio::join!(
        draft::issue(&store, &actor, &company, &b.id, issue_command(key2, 2)),
        draft::update(
            &store,
            &actor,
            &company,
            &b.id,
            Update {
                revision: 2,
                data: next
            }
        )
    );
    assert_eq!(usize::from(issue.is_ok()) + usize::from(edit.is_ok()), 1);
    let value = draft::get(&store, &actor, &company, &b.id).await.unwrap();
    let value = if value.number.is_none() {
        draft::issue(
            &store,
            &actor,
            &company,
            &b.id,
            issue_command(key2, value.revision),
        )
        .await
        .unwrap()
    } else {
        value
    };
    assert_eq!(value.number.as_deref(), Some("FV2027000001"));
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn qr_payment_pdf_uses_exact_issued_snapshot(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    for language in ["cs", "en"] {
        let mut input = issuable();
        input.language = language.into();
        input.payment.method = "bank_transfer".into();
        input.payment.iban = "CZ6508000000192000145399".into();
        input.payment.variable_symbol = "001234".into();
        let saved = draft::create(
            &store,
            &actor,
            &company,
            Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data: input,
            },
        )
        .await
        .unwrap();
        let issued = draft::issue(
            &store,
            &actor,
            &company,
            &saved.id,
            issue_command(&uuid::Uuid::new_v4().to_string(), saved.revision),
        )
        .await
        .unwrap();
        let pdf = draft::pdf(&store, &actor, &company, &issued.id)
            .await
            .unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert_eq!(
            pdf,
            draft::pdf(&store, &actor, &company, &issued.id)
                .await
                .unwrap()
        );
        assert!(draft::pdf(&store, &actor, &other, &issued.id)
            .await
            .is_err());
        std::fs::create_dir_all("../../tmp/pdfs").unwrap();
        std::fs::write(format!("../../tmp/pdfs/qr-{language}.pdf"), pdf).unwrap();
        let expected = faktura_domain::payment_qr::payload(
            &issued.data.payment,
            &issued.totals.as_ref().unwrap().payable,
            "CZK",
            issued.data.due_on,
        )
        .unwrap();
        std::fs::write(format!("../../tmp/pdfs/qr-{language}.txt"), expected).unwrap();
    }
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn foreign_iban_is_issued_and_retained_in_pdf_snapshot(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    for language in ["cs", "en"] {
        let mut input = issuable();
        input.language = language.into();
        input.payment.method = "bank_transfer".into();
        input.payment.bank_account.clear();
        input.payment.iban = "DE89370400440532013000".into();
        let saved = draft::create(
            &store,
            &actor,
            &company,
            Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data: input,
            },
        )
        .await
        .unwrap();
        let issued = draft::issue(
            &store,
            &actor,
            &company,
            &saved.id,
            issue_command(&uuid::Uuid::new_v4().to_string(), saved.revision),
        )
        .await
        .unwrap();
        assert_eq!(issued.data.payment.iban, "DE89370400440532013000");
        assert!(
            faktura_domain::payment_qr::payload(&issued.data.payment, "100.00", "CZK", None)
                .is_none()
        );
        let pdf = draft::pdf(&store, &actor, &company, &issued.id)
            .await
            .unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert_eq!(
            pdf,
            draft::pdf(&store, &actor, &company, &issued.id)
                .await
                .unwrap()
        );
        assert!(draft::pdf(&store, &actor, &other, &issued.id)
            .await
            .is_err());
        std::fs::create_dir_all("../../tmp/pdfs").unwrap();
        std::fs::write(format!("../../tmp/pdfs/foreign-iban-{language}.pdf"), pdf).unwrap();
    }
}

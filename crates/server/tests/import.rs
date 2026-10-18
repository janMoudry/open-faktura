use faktura_application::{
    auth, company,
    import::{self as migration, Apply, Error, ImportStore, Prepare},
};
use faktura_domain::{company::CompanySettings, import::Record};
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, import::PgImport};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn key(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let a = PgAuth::new(
        pool.clone(),
        Some("synthetic-payment-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &a,
        "synthetic-payment-bootstrap-key-only-tests",
        "owner@payments.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (actor, token) = auth::login(&a, "owner@payments.test", "Synthetic password 123!")
        .await
        .unwrap();
    let year: i32 = sqlx::query_scalar("SELECT extract(year FROM now())::integer")
        .fetch_one(pool)
        .await
        .unwrap();
    let s = CompanySettings {
        number_year: year,
        ..Default::default()
    };
    let c = company::create(&PgCompany(pool.clone()), &actor.id, KEY, s.clone())
        .await
        .unwrap();
    let other = company::create(&PgCompany(pool.clone()), &actor.id, &key(2), s)
        .await
        .unwrap();
    (actor.id, token, c.id, other.id)
}

fn records() -> Vec<Record> {
    serde_json::from_str(include_str!("../../../tests/fixtures/import-records.json")).unwrap()
}
fn input(n: u32) -> Prepare {
    Prepare {
        request_id: key(n),
        source: "idoklad".into(),
        agenda: "synthetic-account-1".into(),
        records: records(),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn preview_apply_resume_replay_and_original_amounts(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgImport(pool.clone());
    let preview = migration::prepare(&store, &actor, &company, input(100))
        .await
        .unwrap();
    assert!(
        preview.rows.iter().all(|r| r.status == "ready"),
        "{:?}",
        preview
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let result = migration::apply(
        &store,
        &actor,
        &company,
        &preview.id,
        Apply { skip: vec![] },
    )
    .await
    .unwrap();
    assert_eq!(result.status, "done");
    assert!(result.rows.iter().all(|r| r.status == "imported"));
    assert_eq!(result.totals[0].imported, "59.99");
    assert_eq!(result.totals[0].payments, "20.00");
    let invoice = faktura_application::draft::get(
        &faktura_infrastructure::draft::PgDraft(pool.clone()),
        &actor,
        &company,
        result.rows[1].target_id.as_ref().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(invoice.number.as_deref(), Some("HIST-2025-001"));
    assert_eq!(invoice.totals.unwrap().payable, "59.99");
    assert_eq!(invoice.settlement.unwrap().outstanding, "39.99");
    assert_eq!(invoice.data.contact_id, result.rows[0].target_id);
    let next: i32 = sqlx::query_scalar(
        "SELECT (settings->>'next_number')::int FROM company WHERE id=$1::text::bigint",
    )
    .bind(&company)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(next, 1);
    assert!(faktura_application::draft::pdf(
        &faktura_infrastructure::draft::PgDraft(pool.clone()),
        &actor,
        &company,
        &invoice.id
    )
    .await
    .is_err());
    let replay = migration::prepare(&store, &actor, &company, input(100))
        .await
        .unwrap();
    assert_eq!(replay.status, "done");
    let again = migration::apply(
        &store,
        &actor,
        &company,
        &preview.id,
        Apply { skip: vec![] },
    )
    .await
    .unwrap();
    assert_eq!(again.rows[1].target_id, result.rows[1].target_id);
    let duplicate = migration::prepare(&store, &actor, &company, input(101))
        .await
        .unwrap();
    assert!(duplicate.rows.iter().all(|r| r.status == "duplicate"));
    let duplicate = migration::apply(
        &store,
        &actor,
        &company,
        &duplicate.id,
        Apply { skip: vec![] },
    )
    .await
    .unwrap();
    assert_eq!(duplicate.totals[0].skipped, "59.99");
    assert_eq!(duplicate.totals[0].imported, "0.00");
    assert!(matches!(
        store.get(&actor, &other, &preview.id).await,
        Err(Error::NotFound)
    ));
    let isolated = migration::prepare(&store, &actor, &other, input(102))
        .await
        .unwrap();
    assert!(isolated.rows.iter().all(|r| r.status == "ready"));
    let mut changed = input(103);
    if let Record::Contact { data, .. } = &mut changed.records[0] {
        data.name = "Changed".into();
    }
    let changed = migration::prepare(&store, &actor, &company, changed)
        .await
        .unwrap();
    assert_eq!(changed.rows[0].status, "conflict");
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn conflicts_invalid_currency_missing_reference_and_scopes(pool: PgPool) {
    let (actor, _, company, _) = setup(&pool).await;
    let store = PgImport(pool.clone());
    let mut value = input(200);
    if let Record::Invoice {
        data,
        totals,
        contact_key,
        ..
    } = &mut value.records[1]
    {
        data.calculation.currency = "EUR".into();
        totals.currency = "EUR".into();
        *contact_key = Some("missing".into());
    }
    let preview = migration::prepare(&store, &actor, &company, value)
        .await
        .unwrap();
    assert_eq!(preview.rows[1].status, "invalid");
    assert!(preview.rows[1].fields.contains(&"currency".into()));
    assert_eq!(preview.totals[0].currency, "EUR");
    let blocked = migration::apply(
        &store,
        &actor,
        &company,
        &preview.id,
        Apply { skip: vec![] },
    )
    .await
    .unwrap();
    assert_eq!(blocked.rows[0].status, "ready");
    let result = migration::apply(
        &store,
        &actor,
        &company,
        &preview.id,
        Apply { skip: vec![1] },
    )
    .await
    .unwrap();
    assert_eq!(result.rows[0].status, "imported");
    assert_eq!(result.rows[1].status, "skipped");
    let mut invoice = input(201);
    invoice.records.remove(0);
    if let Record::Invoice { contact_key, .. } = &mut invoice.records[0] {
        *contact_key = Some("absent".into());
    }
    let preview = migration::prepare(&store, &actor, &company, invoice)
        .await
        .unwrap();
    assert!(preview.rows[0].fields.contains(&"contact_key".into()));
    let mut different = input(202);
    different.agenda = "another-source-account".into();
    let preview = migration::prepare(&store, &actor, &company, different)
        .await
        .unwrap();
    assert_eq!(preview.rows[0].status, "conflict");
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&company).bind(&actor).execute(&pool).await.unwrap();
    assert!(matches!(
        migration::prepare(&store, &actor, &company, input(203)).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        migration::apply(
            &store,
            &actor,
            &company,
            &preview.id,
            Apply { skip: vec![] }
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert!(store.get(&actor, &company, &preview.id).await.is_ok());
    sqlx::query(
        "DELETE FROM membership WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint",
    )
    .bind(&company)
    .bind(&actor)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        store.get(&actor, &company, &preview.id).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn concurrent_batches_and_chunked_resume(pool: PgPool) {
    let (actor, _, company, _) = setup(&pool).await;
    let store = PgImport(pool.clone());
    let mut large = input(300);
    large.records = (0..40)
        .map(|n| {
            let mut c = records().remove(0);
            if let Record::Contact { key, data } = &mut c {
                *key = format!("contact-{n}");
                data.name = format!("Contact {n}");
                data.registration_number = format!("REG-{n}");
            }
            c
        })
        .collect();
    let a = migration::prepare(&store, &actor, &company, large.clone())
        .await
        .unwrap();
    large.request_id = key(301);
    let b = migration::prepare(&store, &actor, &company, large)
        .await
        .unwrap();
    let (r1, r2) = tokio::join!(
        migration::apply(&store, &actor, &company, &a.id, Apply { skip: vec![] }),
        migration::apply(&store, &actor, &company, &b.id, Apply { skip: vec![] })
    );
    assert_eq!(r1.unwrap().status, "running");
    assert_eq!(r2.unwrap().status, "running");
    for id in [&a.id, &b.id] {
        let r = migration::apply(&store, &actor, &company, id, Apply { skip: vec![] })
            .await
            .unwrap();
        assert_eq!(r.status, "done");
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM contact")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 40);
    let same = migration::prepare(&store, &actor, &company, input(302))
        .await
        .unwrap();
    let (r1, r2) = tokio::join!(
        migration::apply(&store, &actor, &company, &same.id, Apply { skip: vec![] }),
        migration::apply(&store, &actor, &company, &same.id, Apply { skip: vec![] })
    );
    assert_eq!(r1.unwrap().rows[1].target_id, r2.unwrap().rows[1].target_id);
}
#[test]
fn source_totals_rejected_not_recomputed() {
    let mut r = records();
    assert!(r[1].validate().is_empty());
    if let Record::Invoice { totals, .. } = &mut r[1] {
        totals.payable = "60.00".into();
    }
    assert!(r[1].validate().contains(&"totals.payable".into()));
    let mut r = records();
    if let Record::Invoice { payments, .. } = &mut r[1] {
        payments[0].amount = "0.001".into();
    }
    assert!(!r[1].validate().is_empty());
}
#[test]
fn csv_quotes_bom_limits_and_invalid_encoding() {
    let t = faktura_infrastructure::import_table::parse(
        "csv",
        "\u{feff}ID;Název\r\n001;\"Firma; test\"\r\n".as_bytes(),
        ";",
    )
    .unwrap();
    assert_eq!(t["rows"][0][0], "001");
    assert_eq!(t["rows"][0][1], "Firma; test");
    assert!(faktura_infrastructure::import_table::parse("csv", b"ID;ID\n1;2", ";").is_err());
    assert!(faktura_infrastructure::import_table::parse("csv", b"ID\n\xff", ";").is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn rest_mcp_parity_and_csrf(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (_, token, company, _) = setup(&pool).await;
    let app = faktura_server::import::router(pool.clone(), vec!["https://app.test".into()], false);
    let body = serde_json::to_value(input(400)).unwrap();
    let mut saved = None;
    for (origin, cookie, status) in [
        ("https://evil.test", token.as_str(), 401),
        ("https://app.test", "", 401),
        ("https://app.test", token.as_str(), 200),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/companies/{company}/imports"))
                    .header("origin", origin)
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), status);
        if status == 200 {
            saved = Some(
                serde_json::from_slice::<serde_json::Value>(
                    &to_bytes(r.into_body(), 100000).await.unwrap(),
                )
                .unwrap(),
            );
        }
    }
    let bad = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{company}/imports"))
                .header("origin", "https://app.test")
                .header("cookie", format!("faktura_session={token}"))
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from("{\"unknown\":true}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status().as_u16(), 422);
    let bad: serde_json::Value =
        serde_json::from_slice(&to_bytes(bad.into_body(), 10000).await.unwrap()).unwrap();
    assert_eq!(bad["code"], "invalid_input");
    let money = serde_json::json!({"request_id":key(410),"agenda":"money-transport","domestic_currency":"CZK","xml":include_str!("../../../tests/fixtures/import/money-s3-synthetic.xml")});
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{company}/imports/money"))
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "https://app.test")
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&money).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let money_http: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1000000).await.unwrap()).unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token);
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
    let mut args = body.as_object().unwrap().clone();
    args.insert("company_id".into(), company.clone().into());
    let r = client
        .call_tool(CallToolRequestParams::new("import_prepare").with_arguments(args))
        .await
        .unwrap();
    assert_eq!(r.is_error, Some(false));
    let encoded = serde_json::to_value(r).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(value, saved.unwrap());
    let args = serde_json::json!({"company_id":company,"id":key(400),"skip":[]});
    let r = client
        .call_tool(
            CallToolRequestParams::new("import_apply")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(r.is_error, Some(false));
    let encoded = serde_json::to_value(r).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(value["status"], "done");
    let mut money_args = money.as_object().unwrap().clone();
    money_args.insert("company_id".into(), company.clone().into());
    let result = client
        .call_tool(CallToolRequestParams::new("import_money").with_arguments(money_args))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let result = serde_json::to_value(result).unwrap();
    let money_mcp: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(money_http, money_mcp);
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[test]
fn xlsx_preserves_text_identifiers_and_rejects_formulas_or_huge_ranges() {
    let t = faktura_infrastructure::import_table::parse(
        "xlsx",
        include_bytes!("../../../tests/fixtures/import-contacts.xlsx"),
        ";",
    )
    .unwrap();
    assert_eq!(t["rows"][0][0], "001");
    for file in [
        include_bytes!("../../../tests/fixtures/import-formula.xlsx").as_slice(),
        include_bytes!("../../../tests/fixtures/import-huge-range.xlsx").as_slice(),
    ] {
        assert!(faktura_infrastructure::import_table::parse("xlsx", file, ";").is_err());
    }
}

#[sqlx::test(migrations = false)]
#[ignore = "requires isolated PostgreSQL"]
async fn upgrade_preserves_p08_data(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.migrations
                .iter()
                .filter(|m| m.version < 202610110010)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (actor, _, company, _) = setup(&pool).await;
    let before: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(c) FROM company c WHERE id=$1::text::bigint")
            .bind(&company)
            .fetch_one(&pool)
            .await
            .unwrap();
    all.run(&pool).await.unwrap();
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT to_jsonb(c) FROM company c WHERE id=$1::text::bigint")
            .bind(&company)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    let b = migration::prepare(&PgImport(pool), &actor, &company, input(500))
        .await
        .unwrap();
    assert!(b.rows.iter().all(|r| r.status == "ready"));
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn money_catalog_preview_apply_replay_conflicts_and_scope(pool: PgPool) {
    use faktura_application::import::MoneyPrepare;
    const XML: &str = include_str!("../../../tests/fixtures/import/money-s3-synthetic.xml");
    let (a, _, c, other) = setup(&pool).await;
    let store = PgImport(pool.clone());
    let command = |n, xml: &str| MoneyPrepare {
        request_id: key(n),
        agenda: "synthetic-money".into(),
        domestic_currency: "CZK".into(),
        xml: xml.into(),
    };
    let preview = store
        .prepare_money(&a, &c, command(700, XML))
        .await
        .unwrap();
    assert_eq!(preview.rows[0].status, "ready");
    assert_eq!(preview.rows[0].catalog.as_ref().unwrap().unit_price, "750");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM catalog")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(store.get(&a, &other, &preview.id).await.is_err());
    let second = store
        .prepare_money(&a, &c, command(701, XML))
        .await
        .unwrap();
    let (left, right) = tokio::join!(
        store.apply(&a, &c, &preview.id, Apply { skip: vec![] }),
        store.apply(&a, &c, &second.id, Apply { skip: vec![] })
    );
    let statuses = [
        left.unwrap().rows[0].status.clone(),
        right.unwrap().rows[0].status.clone(),
    ];
    assert!(statuses.contains(&"imported".to_string()));
    assert!(statuses.contains(&"duplicate".to_string()));
    store
        .apply(&a, &c, &preview.id, Apply { skip: vec![] })
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM catalog")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM catalog_audit")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stock_card")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let changed = store
        .prepare_money(&a, &c, command(702, &XML.replace("750", "751")))
        .await
        .unwrap();
    assert_eq!(changed.rows[0].status, "conflict");
    let duplicate = store
        .prepare_money(&a, &c, command(703, XML))
        .await
        .unwrap();
    assert_eq!(duplicate.rows[0].status, "duplicate");
    let different_key = XML.replace("8000-000000000001", "8000-000000000003");
    let collision = store
        .prepare_money(&a, &c, command(704, &different_key))
        .await
        .unwrap();
    assert_eq!(collision.rows[0].status, "conflict");
    let invalid = store
        .prepare_money(
            &a,
            &c,
            command(705, &different_key.replace("<SDPH_Prod>21", "<SDPH_Prod>5")),
        )
        .await
        .unwrap();
    assert_eq!(invalid.rows[0].status, "invalid");
    assert_eq!(
        store
            .apply(&a, &c, &invalid.id, Apply { skip: vec![0] })
            .await
            .unwrap()
            .status,
        "done"
    );
    assert_eq!(
        store
            .prepare_money(&a, &other, command(706, XML))
            .await
            .unwrap()
            .rows[0]
            .status,
        "ready"
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1 AND user_id=$2")
        .bind(c.parse::<i64>().unwrap())
        .bind(a.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store.prepare_money(&a, &c, command(707, XML)).await,
        Err(Error::Forbidden)
    ));
    assert!(store.get(&a, &c, &preview.id).await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn generic_catalog_invalid_rows_and_exact_values(pool: PgPool) {
    use faktura_domain::{catalog::CatalogData, invoice::PriceMode};
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgImport(pool.clone());
    let valid = CatalogData {
        name: "Mapped synthetic service".into(),
        code: "0002".into(),
        barcode: "0003".into(),
        unit: "h".into(),
        quantity: "1".into(),
        unit_price: "1250.5000".into(),
        currency: "CZK".into(),
        price_mode: PriceMode::Net,
        vat_percent: "21".into(),
    };
    let mut invalid = valid.clone();
    invalid.unit_price = "invalid".into();
    invalid.currency = "EUR".into();
    invalid.vat_percent = "5".into();
    let command = |n, records| Prepare {
        request_id: key(n),
        source: "generic".into(),
        agenda: "mapped-test".into(),
        records,
    };
    let record = Record::Catalog {
        key: "0001".into(),
        data: Box::new(valid),
    };
    let preview = migration::prepare(
        &store,
        &actor,
        &company,
        command(
            800,
            vec![
                record.clone(),
                Record::Catalog {
                    key: "invalid".into(),
                    data: Box::new(invalid),
                },
            ],
        ),
    )
    .await
    .unwrap();
    assert_eq!(preview.rows[0].status, "ready");
    assert_eq!(preview.rows[1].status, "invalid");
    for field in ["unit_price", "currency", "vat_percent"] {
        assert!(preview.rows[1].fields.iter().any(|f| f == field));
    }
    let blocked = store
        .apply(&actor, &company, &preview.id, Apply { skip: vec![] })
        .await
        .unwrap();
    assert_eq!(blocked.rows[0].status, "ready");
    assert_eq!(blocked.rows[1].status, "invalid");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM catalog")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(store
        .apply(&actor, &other, &preview.id, Apply { skip: vec![1] })
        .await
        .is_err());
    let done = store
        .apply(&actor, &company, &preview.id, Apply { skip: vec![1] })
        .await
        .unwrap();
    assert_eq!(done.rows[0].status, "imported");
    let data: serde_json::Value =
        sqlx::query_scalar("SELECT data FROM catalog WHERE company_id=$1::text::bigint")
            .bind(&company)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(data["unit_price"], "1250.5");
    assert_eq!(data["barcode"], "0003");
    assert_eq!(data["quantity"], "1");
    assert_eq!(data["price_mode"], "net");
    let replay = migration::prepare(&store, &actor, &company, command(801, vec![record]))
        .await
        .unwrap();
    assert_eq!(replay.rows[0].status, "duplicate");
}

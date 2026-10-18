use faktura_application::{
    auth,
    catalog::{self, Create},
    company,
    stock::{self, StockStore},
};
use faktura_domain::{catalog::CatalogData, company::CompanySettings};
use faktura_infrastructure::{
    auth::PgAuth, catalog::PgCatalog, company::PgCompany, stock::PgStock,
};
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
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn cmd(i: &str, action: stock::Action) -> stock::Command {
    stock::Command {
        acquisition_unit_cost: None,
        request_id: key(),
        catalog_id: i.into(),
        action,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn exact_inventory_replay_reversal_counts_and_isolation(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgStock(p.clone());
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Synthetic stock", &key()),
    )
    .await
    .unwrap();
    let open = cmd(
        &item.id,
        stock::Action::Open {
            quantity: "10.1234".into(),
            booked_on: "2026-10-01".parse().unwrap(),
            reason: "Opening".into(),
        },
    );
    let json = serde_json::to_value(&open).unwrap();
    let opened = s.execute(&a, &c, open).await.unwrap();
    assert_eq!(
        s.execute(&a, &c, serde_json::from_value(json).unwrap())
            .await
            .unwrap()
            .id,
        opened.id
    );
    assert!(s
        .execute(
            &a,
            &other,
            cmd(
                &item.id,
                stock::Action::Move {
                    quantity: "1".into(),
                    booked_on: "2026-10-15".parse().unwrap(),
                    reason: "wrong profile".into()
                }
            )
        )
        .await
        .is_err());
    let moved = s
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Move {
                    quantity: "-0.1234".into(),
                    booked_on: "2026-10-15".parse().unwrap(),
                    reason: "Issue".into(),
                },
            ),
        )
        .await
        .unwrap();
    let card = s.cards(&a, &c).await.unwrap().pop().unwrap();
    assert_eq!(card.quantity, "10");
    let counted = s
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Count {
                    quantity: "9.5".into(),
                    revision: card.revision,
                    reason: "Physical count".into(),
                },
            ),
        )
        .await
        .unwrap();
    assert_eq!(counted.quantity, "-0.5");
    assert!(s
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Count {
                    quantity: "10".into(),
                    revision: card.revision,
                    reason: "Stale".into()
                }
            )
        )
        .await
        .is_err());
    s.execute(
        &a,
        &c,
        cmd(
            &item.id,
            stock::Action::Reverse {
                movement_id: moved.id.clone(),
                reason: "Return".into(),
            },
        ),
    )
    .await
    .unwrap();
    assert!(s
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Reverse {
                    movement_id: moved.id,
                    reason: "Duplicate".into()
                }
            )
        )
        .await
        .is_err());
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "9.6234");
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        4
    );
    let history = s
        .movements(
            &a,
            &c,
            stock::Search {
                catalog_id: Some(item.id.clone()),
                from: Some("2026-10-02".parse().unwrap()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let summary = history.summary.unwrap();
    assert_eq!(summary.opening, "10.1234");
    assert_eq!(summary.received, "0");
    assert_eq!(summary.issued, "0.1234");
    assert_eq!(summary.adjustments, "-0.3766");
    assert_eq!(summary.closing, "9.6234");
    for _ in 0..26 {
        s.execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Move {
                    quantity: "0.0001".into(),
                    booked_on: "2026-10-15".parse().unwrap(),
                    reason: "Receipt".into(),
                },
            ),
        )
        .await
        .unwrap();
    }
    let second = s
        .movements(
            &a,
            &c,
            stock::Search {
                catalog_id: Some(item.id.clone()),
                page: Some(2),
                from: Some("2026-10-02".parse().unwrap()),
                to: Some("2026-10-15".parse().unwrap()),
            },
        )
        .await
        .unwrap();
    assert_eq!(second.total, 27);
    assert_eq!(second.items.len(), 2);
    let summary = second.summary.unwrap();
    assert_eq!(summary.received, "0.0026");
    assert_eq!(summary.closing, "10.0026");
    assert_eq!(summary.adjustments, "0");
    assert!(s
        .movements(
            &a,
            &other,
            stock::Search {
                catalog_id: Some(item.id.clone()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .summary
        .is_none());
    let mut changed = item.data.clone();
    changed.unit = "kg".into();
    assert!(catalog::update(
        &PgCatalog(p.clone()),
        &a,
        &c,
        &item.id,
        catalog::Update {
            revision: item.revision,
            archived: false,
            data: changed
        }
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_count_one_wins_validation_and_viewer(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgStock(p.clone());
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Synthetic stock", &key()),
    )
    .await
    .unwrap();
    assert!(s
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Open {
                    quantity: "1.00001".into(),
                    booked_on: "2026-10-01".parse().unwrap(),
                    reason: "Invalid precision".into()
                }
            )
        )
        .await
        .is_err());
    assert!(s.cards(&a, &c).await.unwrap().is_empty());
    s.execute(
        &a,
        &c,
        cmd(
            &item.id,
            stock::Action::Open {
                quantity: "0".into(),
                booked_on: "2026-10-01".parse().unwrap(),
                reason: "Opening".into(),
            },
        ),
    )
    .await
    .unwrap();
    let card = s.cards(&a, &c).await.unwrap().pop().unwrap();
    let one = cmd(
        &item.id,
        stock::Action::Count {
            quantity: "7".into(),
            revision: card.revision,
            reason: "One".into(),
        },
    );
    let two = cmd(
        &item.id,
        stock::Action::Count {
            quantity: "8".into(),
            revision: card.revision,
            reason: "Two".into(),
        },
    );
    let (l, r) = tokio::join!(s.execute(&a, &c, one), s.execute(&a, &c, two));
    assert_eq!(usize::from(l.is_ok()) + usize::from(r.is_ok()), 1);
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.cards(&a, &c).await.is_ok());
    assert!(s
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Move {
                    quantity: "1".into(),
                    booked_on: "2026-10-15".parse().unwrap(),
                    reason: "Denied".into()
                }
            )
        )
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_statistics_match_and_origin_is_required(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let s = PgStock(p.clone());
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Synthetic stock", &key()),
    )
    .await
    .unwrap();
    s.execute(
        &a,
        &c,
        cmd(
            &item.id,
            stock::Action::Open {
                quantity: "1.25".into(),
                booked_on: "2026-10-01".parse().unwrap(),
                reason: "Opening".into(),
            },
        ),
    )
    .await
    .unwrap();
    let router =
        faktura_server::stock::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/stock/movements?catalog_id={}",
                    item.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1000000).await.unwrap()).unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(p.clone(), token.clone());
    let task = tokio::spawn(async move {
        server
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap()
    });
    let client = ().serve(client_io).await.unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("stock_movements").with_arguments(
                serde_json::json!({"company_id":c,"catalog_id":item.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let json = serde_json::to_value(result).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http, mcp);
    assert_eq!(http["summary"]["closing"], "1.25");
    assert_eq!(http["summary"]["adjustments"], "1.25");
    let report_http = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/stock/report?valuation=automatic"
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(report_http.status(), 200);
    let report_http: serde_json::Value =
        serde_json::from_slice(&to_bytes(report_http.into_body(), 1000000).await.unwrap()).unwrap();
    let report_mcp = client
        .call_tool(
            CallToolRequestParams::new("stock_report").with_arguments(
                serde_json::json!({"company_id":c,"valuation":"automatic"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    let report_mcp = serde_json::to_value(report_mcp).unwrap();
    let report_mcp: serde_json::Value =
        serde_json::from_str(report_mcp["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report_http, report_mcp);
    let cost=client.call_tool(CallToolRequestParams::new("stock_set_cost").with_arguments(serde_json::json!({"company_id":c,"catalog_id":item.id,"request_id":key(),"revision":0,"effective_on":"2026-10-01","unit_cost":"2","reason":"Synthetic valuation"}).as_object().unwrap().clone())).await.unwrap();
    assert_ne!(cost.is_error, Some(true));
    let costs = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/stock/costs/{}", item.id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let costs: serde_json::Value =
        serde_json::from_slice(&to_bytes(costs.into_body(), 1000000).await.unwrap()).unwrap();
    assert_eq!(costs[0]["unit_cost"], "2");
    let result = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/stock/commands"))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .header("origin", "https://untrusted.example.test")
                .body(Body::from(
                    serde_json::to_vec(&cmd(
                        &item.id,
                        stock::Action::Move {
                            quantity: "5".into(),
                            booked_on: "2026-10-15".parse().unwrap(),
                            reason: "Rejected".into(),
                        },
                    ))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(result.status(), 401);
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "1.25");
    let http = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/stock/sales-report"))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(http.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(http.into_body(), 1000000).await.unwrap()).unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("item_sales_report").with_arguments(
                serde_json::json!({"company_id":c})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let result = serde_json::to_value(result).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http, mcp);
    let export = client
        .call_tool(
            CallToolRequestParams::new("item_sales_export").with_arguments(
                serde_json::json!({"company_id":c,"search":{},"format":"csv","language":"cs"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(export.is_error, Some(true));
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn document_stock_issue_purchase_correction_and_credit(p: PgPool) {
    use faktura_application::{credit_note, draft, received};
    use faktura_infrastructure::{credit_note::PgCredit, draft::PgDraft, received::PgReceived};
    let (a, _, c, other) = setup(&p).await;
    let s = PgStock(p.clone());
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Stock product", &key()),
    )
    .await
    .unwrap();
    s.execute(
        &a,
        &c,
        cmd(
            &item.id,
            stock::Action::Open {
                quantity: "10".into(),
                booked_on: "2026-10-01".parse().unwrap(),
                reason: "Opening".into(),
            },
        ),
    )
    .await
    .unwrap();
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.payment.method = "cash".into();
    data.supplier.registration_number = "12345678".into();
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    data.calculation.lines[0].vat_percent = "0".into();
    data.calculation.lines.truncate(1);
    data.calculation.lines[0].catalog_id = Some(item.id.clone());
    data.calculation.lines[0].unit = item.data.unit.clone();
    data.calculation.lines[0].quantity = "2.125".into();
    let draft = draft::create(
        &PgDraft(p.clone()),
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data: data.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "10");
    let issue_key = key();
    let issued = draft::issue(
        &PgDraft(p.clone()),
        &a,
        &c,
        &draft.id,
        draft::Issue {
            request_id: issue_key.clone(),
            revision: draft.revision,
        },
    )
    .await
    .unwrap();
    draft::issue(
        &PgDraft(p.clone()),
        &a,
        &c,
        &draft.id,
        draft::Issue {
            request_id: issue_key,
            revision: draft.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "7.875");
    assert!(draft::create(
        &PgDraft(p.clone()),
        &a,
        &other,
        draft::Create {
            request_id: key(),
            data: data.clone()
        }
    )
    .await
    .is_err());
    let purchase = faktura_domain::received::Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Default::default(),
        receipt_method: None,
        supplier_contact_id: None,
        supplier: data.supplier.clone(),
        external_number: "SUP-1".into(),
        variable_symbol: "".into(),
        bank_account: "".into(),
        issued_on: "2026-10-01".parse().unwrap(),
        received_on: "2026-10-01".parse().unwrap(),
        due_on: "2026-10-31".parse().unwrap(),
        tax_on: None,
        currency: "CZK".into(),
        description: "Purchase".into(),
        note: "".into(),
        rounding: "0".into(),
        lines: vec![faktura_domain::received::Line {
            description: "Product".into(),
            rate: "0".into(),
            net: "100".into(),
            vat: "0".into(),
            stock: Some(faktura_domain::received::StockReceipt {
                acquisition_unit_cost: None,
                catalog_id: item.id.clone(),
                quantity: "3.25".into(),
                unit: item.data.unit.clone(),
            }),
        }],
    };
    let request = key();
    let bought = received::create(
        &PgReceived(p.clone()),
        &a,
        &c,
        received::Create {
            request_id: request.clone(),
            data: purchase.clone(),
        },
    )
    .await
    .unwrap();
    received::create(
        &PgReceived(p.clone()),
        &a,
        &c,
        received::Create {
            request_id: request,
            data: purchase.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "11.125");
    let mut updated = purchase.clone();
    updated.lines[0].stock.as_mut().unwrap().quantity = "1.5".into();
    let updated = received::command(
        &PgReceived(p.clone()),
        &a,
        &c,
        &bought.id,
        received::Command {
            request_id: key(),
            revision: bought.revision,
            action: received::Action::Update {
                data: Box::new(updated),
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "9.375");
    received::command(
        &PgReceived(p.clone()),
        &a,
        &c,
        &bought.id,
        received::Command {
            request_id: key(),
            revision: updated.revision,
            action: received::Action::Cancel {
                reason: "Return purchase".into(),
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "7.875");
    let credit = credit_note::create(
        &PgCredit(p.clone()),
        &a,
        &c,
        credit_note::Create {
            request_id: key(),
            invoice_id: issued.id.clone(),
        },
    )
    .await
    .unwrap();
    let mut correction = credit.data.clone();
    correction.reason = "Returned goods".into();
    correction.lines[0].amount = "1".into();
    correction.lines[0].return_quantity = Some("1.125".into());
    let credit = credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &credit.id,
        credit_note::Command {
            request_id: key(),
            revision: credit.revision,
            action: credit_note::Action::Update { data: correction },
        },
    )
    .await
    .unwrap();
    credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &credit.id,
        credit_note::Command {
            request_id: key(),
            revision: credit.revision,
            action: credit_note::Action::Issue,
        },
    )
    .await
    .unwrap();
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "9");
    let excessive = credit_note::create(
        &PgCredit(p.clone()),
        &a,
        &c,
        credit_note::Create {
            request_id: key(),
            invoice_id: issued.id.clone(),
        },
    )
    .await
    .unwrap();
    let mut d = excessive.data.clone();
    d.reason = "Too many returns".into();
    d.lines[0].return_quantity = Some("2".into());
    let excessive = credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &excessive.id,
        credit_note::Command {
            request_id: key(),
            revision: excessive.revision,
            action: credit_note::Action::Update { data: d },
        },
    )
    .await
    .unwrap();
    assert!(credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &excessive.id,
        credit_note::Command {
            request_id: key(),
            revision: excessive.revision,
            action: credit_note::Action::Issue
        }
    )
    .await
    .is_err());
    assert_eq!(s.cards(&a, &c).await.unwrap()[0].quantity, "9");
    let history = s
        .movements(
            &a,
            &c,
            stock::Search {
                catalog_id: Some(item.id.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(history
        .items
        .iter()
        .any(|m| m.source_id.as_deref() == Some(issued.id.as_str()) && m.kind == "invoice"));
    assert_eq!(history.summary.unwrap().closing, "9");
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_043_keeps_manual_history(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610170044)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let item = catalog::create(&PgCatalog(p.clone()), &a, &c, create("Old card", &key()))
        .await
        .unwrap();
    sqlx::query("INSERT INTO stock_card(company_id,catalog_id,opened_on,quantity) VALUES($1::text::bigint,$2::text::bigint,'2026-10-01',4.25)").bind(&c).bind(&item.id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO stock_movement(company_id,catalog_id,quantity,booked_on,kind,reason,item,actor_id) VALUES($1::text::bigint,$2::text::bigint,4.25,'2026-10-01','opening','Old opening',$3,$4::text::bigint)").bind(&c).bind(&item.id).bind(serde_json::to_value(&item.data).unwrap()).bind(&a).execute(&p).await.unwrap();
    faktura_infrastructure::migrate(&p).await.unwrap();
    let h = PgStock(p.clone())
        .movements(
            &a,
            &c,
            stock::Search {
                catalog_id: Some(item.id),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(h.total, 1);
    assert!(h.items[0].source_id.is_none());
    assert_eq!(h.summary.unwrap().closing, "4.25");
    use faktura_application::stock_report::StockReportStore;
    let report = faktura_infrastructure::stock_report::PgStockReport(p)
        .report(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(report.unvalued_count, 1);
    assert_eq!(report.known_value, "0.00");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn stock_valuation_dates_precision_replay_roles_and_pagination(p: PgPool) {
    use faktura_application::stock_report::{CostCommand, Search, StockReportStore};
    use faktura_infrastructure::stock_report::PgStockReport;
    let (a, _, c, other) = setup(&p).await;
    let s = PgStock(p.clone());
    let r = PgStockReport(p.clone());
    let mut ids = vec![];
    for n in 0..27 {
        let item = catalog::create(
            &PgCatalog(p.clone()),
            &a,
            &c,
            create(&format!("Valuation {n:02}"), &key()),
        )
        .await
        .unwrap();
        s.execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Open {
                    quantity: if n == 26 { "-1" } else { "2.5" }.into(),
                    booked_on: "2026-10-01".parse().unwrap(),
                    reason: "Opening".into(),
                },
            ),
        )
        .await
        .unwrap();
        ids.push(item.id);
    }
    let command = CostCommand {
        request_id: key(),
        catalog_id: ids[0].clone(),
        revision: 0,
        effective_on: "2026-10-01".parse().unwrap(),
        unit_cost: Some("1.234".into()),
        reason: "Initial cost".into(),
    };
    let input = serde_json::to_value(&command).unwrap();
    assert_eq!(r.set_cost(&a, &c, command).await.unwrap().revision, 1);
    assert_eq!(
        r.set_cost(&a, &c, serde_json::from_value(input).unwrap())
            .await
            .unwrap()
            .revision,
        1
    );
    let q = || Search {
        as_of: Some("2026-10-01".parse().unwrap()),
        page: Some(2),
        ..Default::default()
    };
    let report = r.report(&a, &c, q()).await.unwrap();
    assert_eq!(report.items.len(), 2);
    assert_eq!(report.total, 27);
    assert_eq!(report.known_value, "3.09");
    assert_eq!(report.unvalued_count, 26);
    assert_eq!(report.negative_count, 1);
    let set = |revision, price: &str| CostCommand {
        request_id: key(),
        catalog_id: ids[0].clone(),
        revision,
        effective_on: "2026-10-02".parse().unwrap(),
        unit_cost: Some(price.into()),
        reason: "Revaluation".into(),
    };
    let (left, right) = tokio::join!(
        r.set_cost(&a, &c, set(1, "2")),
        r.set_cost(&a, &c, set(1, "2"))
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert_eq!(r.report(&a, &c, q()).await.unwrap().known_value, "3.09");
    assert_eq!(
        r.report(
            &a,
            &c,
            Search {
                as_of: Some("2026-10-02".parse().unwrap()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .known_value,
        "5.00"
    );
    assert_eq!(r.costs(&a, &c, &ids[0]).await.unwrap().len(), 2);
    assert!(r.set_cost(&a, &other, set(2, "3")).await.is_err());
    assert_eq!(
        r.report(&a, &other, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
    assert!(r.set_cost(&a, &c, set(2, "0.00001")).await.is_err());
    let mut clear = set(2, "0");
    clear.unit_cost = None;
    r.set_cost(&a, &c, clear).await.unwrap();
    assert_eq!(
        r.report(&a, &c, Default::default())
            .await
            .unwrap()
            .unvalued_count,
        27
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(r.report(&a, &c, Default::default()).await.is_ok());
    assert!(r.set_cost(&a, &c, set(3, "1")).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn automatic_acquisition_average_and_price_only_correction(p: PgPool) {
    use faktura_application::{
        received,
        stock_report::{Search, StockReportStore, Valuation},
    };
    use faktura_infrastructure::{received::PgReceived, stock_report::PgStockReport};
    let (a, _, c, other) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Costed goods", &key()),
    )
    .await
    .unwrap();
    let stock = PgStock(p.clone());
    stock
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Open {
                    quantity: "0".into(),
                    booked_on: "2026-10-01".parse().unwrap(),
                    reason: "Empty opening".into(),
                },
            ),
        )
        .await
        .unwrap();
    let purchase = faktura_domain::received::Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Default::default(),
        receipt_method: None,
        supplier_contact_id: None,
        supplier: faktura_domain::invoice::PartySnapshot {
            name: "Vendor".into(),
            registration_number: "".into(),
            vat_number: "".into(),
            address: faktura_domain::contact::Address::default(),
            email: "".into(),
            phone: "".into(),
        },
        external_number: "SUP-1".into(),
        variable_symbol: "".into(),
        bank_account: "".into(),
        issued_on: "2026-10-01".parse().unwrap(),
        received_on: "2026-10-01".parse().unwrap(),
        due_on: "2026-10-31".parse().unwrap(),
        tax_on: None,
        currency: "CZK".into(),
        description: "Purchase".into(),
        note: "".into(),
        rounding: "0".into(),
        lines: vec![faktura_domain::received::Line {
            description: "Product".into(),
            rate: "0".into(),
            net: "100".into(),
            vat: "0".into(),
            stock: Some(faktura_domain::received::StockReceipt {
                acquisition_unit_cost: Some("20".into()),
                catalog_id: item.id.clone(),
                quantity: "10".into(),
                unit: item.data.unit.clone(),
            }),
        }],
    };

    let store = PgReceived(p.clone());
    let first = received::create(
        &store,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: purchase.clone(),
        },
    )
    .await
    .unwrap();
    let mut next = purchase.clone();
    next.external_number = "SUP-2".into();
    let line = next.lines[0].stock.as_mut().unwrap();
    line.quantity = "5".into();
    line.acquisition_unit_cost = Some("50".into());
    let k = key();
    for _ in 0..2 {
        received::create(
            &store,
            &a,
            &c,
            received::Create {
                request_id: k.clone(),
                data: next.clone(),
            },
        )
        .await
        .unwrap();
    }
    let today = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
        .fetch_one(&p)
        .await
        .unwrap();
    stock
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Move {
                    quantity: "-3".into(),
                    booked_on: today,
                    reason: "Dispatch".into(),
                },
            ),
        )
        .await
        .unwrap();
    let reports = PgStockReport(p.clone());
    let q = || Search {
        valuation: Valuation::Automatic,
        ..Default::default()
    };
    let report = reports.report(&a, &c, q()).await.unwrap();
    assert_eq!(report.known_value, "360.00");
    assert_eq!(report.items[0].quantity, "12");
    assert_eq!(report.items[0].unit_cost.as_deref(), Some("30"));
    assert_eq!(reports.report(&a, &other, q()).await.unwrap().total, 0);
    let mut bad = purchase.clone();
    bad.lines[0].stock.as_mut().unwrap().acquisition_unit_cost = Some("0.00001".into());
    assert!(received::create(
        &store,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: bad
        }
    )
    .await
    .is_err());
    assert_eq!(stock.cards(&a, &c).await.unwrap()[0].quantity, "12");
    for (cost, unit) in [("0.001", Some("20")), ("1", None), ("-1", Some("20"))] {
        let mut bad = purchase.clone();
        bad.stock_incidental_cost = Some(cost.into());
        bad.lines[0].stock.as_mut().unwrap().acquisition_unit_cost = unit.map(str::to_owned);
        assert!(received::create(
            &store,
            &a,
            &c,
            received::Create {
                request_id: key(),
                data: bad
            }
        )
        .await
        .is_err());
    }
    assert!(received::create(
        &store,
        &a,
        &other,
        received::Create {
            request_id: key(),
            data: purchase.clone()
        }
    )
    .await
    .is_err());
    let mut changed = purchase;
    changed.lines[0]
        .stock
        .as_mut()
        .unwrap()
        .acquisition_unit_cost = Some("21".into());
    let command = received::Command {
        request_id: key(),
        revision: first.revision,
        action: received::Action::Update {
            data: Box::new(changed.clone()),
        },
    };
    let replay = serde_json::to_value(&command).unwrap();
    received::command(&store, &a, &c, &first.id, command)
        .await
        .unwrap();
    received::command(
        &store,
        &a,
        &c,
        &first.id,
        serde_json::from_value(replay).unwrap(),
    )
    .await
    .unwrap();
    let after = reports.report(&a, &c, q()).await.unwrap();
    assert_eq!(after.unvalued_count, 0);
    assert_eq!(after.items[0].value.as_deref(), Some("368.00"));
    assert_eq!(after.items[0].issued_value.as_deref(), Some("92.00"));
    assert_eq!(
        after.items[0].correction_stock_value.as_deref(),
        Some("8.00")
    );
    assert_eq!(
        after.items[0].correction_issued_value.as_deref(),
        Some("2.00")
    );
    assert_eq!(after.items[0].quantity, "12");
    let changes:i64=sqlx::query_scalar("SELECT count(*) FROM stock_movement WHERE company_id=$1::text::bigint AND acquisition_correction").bind(&c).fetch_one(&p).await.unwrap();
    assert_eq!(changes, 1);
    changed.stock_incidental_cost = Some("30.00".into());
    received::command(
        &store,
        &a,
        &c,
        &first.id,
        received::Command {
            request_id: key(),
            revision: first.revision + 1,
            action: received::Action::Update {
                data: Box::new(changed),
            },
        },
    )
    .await
    .unwrap();
    let costed = reports.report(&a, &c, q()).await.unwrap();
    assert_eq!(costed.items[0].value.as_deref(), Some("392.00"));
    assert_eq!(costed.items[0].issued_value.as_deref(), Some("98.00"));
    assert_eq!(
        costed.items[0].correction_stock_value.as_deref(),
        Some("32.00")
    );
    assert_eq!(
        costed.items[0].correction_issued_value.as_deref(),
        Some("8.00")
    );
    let history = stock.movements(&a, &c, Default::default()).await.unwrap();
    assert_eq!(
        history.items[0].acquisition_extra_cost.as_deref(),
        Some("30")
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn costed_credit_return_uses_corrected_invoice_issue(p: PgPool) {
    use faktura_application::{
        credit_note, draft, received,
        stock_report::{Search, StockReportStore, Valuation},
    };
    use faktura_infrastructure::{
        credit_note::PgCredit, draft::PgDraft, received::PgReceived, stock_report::PgStockReport,
    };
    let (a, _, c, other) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Returned goods", &key()),
    )
    .await
    .unwrap();
    PgStock(p.clone())
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Open {
                    quantity: "0".into(),
                    booked_on: "2026-10-01".parse().unwrap(),
                    reason: "Opening".into(),
                },
            ),
        )
        .await
        .unwrap();
    let purchase = faktura_domain::received::Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Default::default(),
        receipt_method: None,
        supplier_contact_id: None,
        supplier: faktura_domain::invoice::PartySnapshot {
            name: "Vendor".into(),
            registration_number: "".into(),
            vat_number: "".into(),
            address: faktura_domain::contact::Address::default(),
            email: "".into(),
            phone: "".into(),
        },
        external_number: "SUP-1".into(),
        variable_symbol: "".into(),
        bank_account: "".into(),
        issued_on: "2026-10-01".parse().unwrap(),
        received_on: "2026-10-01".parse().unwrap(),
        due_on: "2026-10-31".parse().unwrap(),
        tax_on: None,
        currency: "CZK".into(),
        description: "Purchase".into(),
        note: "".into(),
        rounding: "0".into(),
        lines: vec![faktura_domain::received::Line {
            description: "Product".into(),
            rate: "0".into(),
            net: "100".into(),
            vat: "0".into(),
            stock: Some(faktura_domain::received::StockReceipt {
                acquisition_unit_cost: Some("20".into()),
                catalog_id: item.id.clone(),
                quantity: "10".into(),
                unit: item.data.unit.clone(),
            }),
        }],
    };

    let source = received::create(
        &PgReceived(p.clone()),
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: purchase.clone(),
        },
    )
    .await
    .unwrap();
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.payment.method = "cash".into();
    data.supplier.registration_number = "12345678".into();
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    data.calculation.lines.truncate(1);
    data.calculation.lines[0].vat_percent = "0".into();
    data.calculation.lines[0].quantity = "4".into();
    data.calculation.lines[0].catalog_id = Some(item.id.clone());
    data.calculation.lines[0].unit = item.data.unit.clone();
    data.issued_on = Some(
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&p)
            .await
            .unwrap(),
    );
    let invoice = draft::create(
        &PgDraft(p.clone()),
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data,
        },
    )
    .await
    .unwrap();
    let invoice = draft::issue(
        &PgDraft(p.clone()),
        &a,
        &c,
        &invoice.id,
        draft::Issue {
            request_id: key(),
            revision: invoice.revision,
        },
    )
    .await
    .unwrap();
    let mut changed = purchase;
    changed.lines[0]
        .stock
        .as_mut()
        .unwrap()
        .acquisition_unit_cost = Some("30".into());
    changed.stock_incidental_cost = Some("20".into());
    received::command(
        &PgReceived(p.clone()),
        &a,
        &c,
        &source.id,
        received::Command {
            request_id: key(),
            revision: source.revision,
            action: received::Action::Update {
                data: Box::new(changed),
            },
        },
    )
    .await
    .unwrap();
    let credit = credit_note::create(
        &PgCredit(p.clone()),
        &a,
        &c,
        credit_note::Create {
            request_id: key(),
            invoice_id: invoice.id,
        },
    )
    .await
    .unwrap();
    let mut returned = credit.data.clone();
    returned.reason = "Physical return".into();
    returned.lines[0].amount = "1".into();
    returned.lines[0].return_quantity = Some("2".into());
    returned.issued_on = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
        .fetch_one(&p)
        .await
        .unwrap();
    let credit = credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &credit.id,
        credit_note::Command {
            request_id: key(),
            revision: credit.revision,
            action: credit_note::Action::Update { data: returned },
        },
    )
    .await
    .unwrap();
    let command = credit_note::Command {
        request_id: key(),
        revision: credit.revision,
        action: credit_note::Action::Issue,
    };
    let replay = serde_json::to_value(&command).unwrap();
    credit_note::command(&PgCredit(p.clone()), &a, &c, &credit.id, command)
        .await
        .unwrap();
    credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &credit.id,
        serde_json::from_value(replay).unwrap(),
    )
    .await
    .unwrap();
    let report = PgStockReport(p.clone())
        .report(
            &a,
            &c,
            Search {
                valuation: Valuation::Automatic,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(report.items[0].quantity, "8");
    assert_eq!(report.items[0].value.as_deref(), Some("256.00"));
    {
        use faktura_application::item_sales::ItemSalesStore;
        let sales = faktura_infrastructure::item_sales::PgItemSales(p.clone())
            .report(&a, &c, Default::default())
            .await
            .unwrap();
        assert_eq!(sales.items[0].quantity, "2");
        assert_eq!(sales.items[0].gross, "78.98");
    }

    assert_eq!(report.items[0].issued_value.as_deref(), Some("64.00"));
    assert_eq!(
        report.items[0].correction_stock_value.as_deref(),
        Some("96.00")
    );
    assert_eq!(
        report.items[0].correction_issued_value.as_deref(),
        Some("24.00")
    );
    assert_eq!(
        PgStockReport(p)
            .report(
                &a,
                &other,
                Search {
                    valuation: Valuation::Automatic,
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .total,
        0
    );
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_046_restates_existing_price_corrections(p: PgPool) {
    use faktura_application::{
        received,
        stock_report::{Search, StockReportStore, Valuation},
    };
    use faktura_infrastructure::{received::PgReceived, stock_report::PgStockReport};
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610170047)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Old priced receipt", &key()),
    )
    .await
    .unwrap();
    let mut purchase = faktura_domain::received::Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Default::default(),
        receipt_method: None,
        supplier_contact_id: None,
        supplier: faktura_domain::invoice::PartySnapshot {
            name: "Vendor".into(),
            registration_number: "".into(),
            vat_number: "".into(),
            address: faktura_domain::contact::Address::default(),
            email: "".into(),
            phone: "".into(),
        },
        external_number: "SUP-1".into(),
        variable_symbol: "".into(),
        bank_account: "".into(),
        issued_on: "2026-10-01".parse().unwrap(),
        received_on: "2026-10-01".parse().unwrap(),
        due_on: "2026-10-31".parse().unwrap(),
        tax_on: None,
        currency: "CZK".into(),
        description: "Purchase".into(),
        note: "".into(),
        rounding: "0".into(),
        lines: vec![faktura_domain::received::Line {
            description: "Product".into(),
            rate: "0".into(),
            net: "100".into(),
            vat: "0".into(),
            stock: Some(faktura_domain::received::StockReceipt {
                acquisition_unit_cost: Some("20".into()),
                catalog_id: item.id.clone(),
                quantity: "10".into(),
                unit: item.data.unit.clone(),
            }),
        }],
    };

    purchase.lines[0].stock = None;
    let source = received::create(
        &PgReceived(p.clone()),
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: purchase,
        },
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO stock_card(company_id,catalog_id,opened_on,quantity) VALUES($1::text::bigint,$2::text::bigint,'2026-10-01',6)").bind(&c).bind(&item.id).execute(&p).await.unwrap();
    for (qty, kind, price, correction) in [
        ("10", "received", Some("20"), false),
        ("-4", "manual", None, false),
        ("0", "received", Some("30"), true),
    ] {
        sqlx::query("INSERT INTO stock_movement(company_id,catalog_id,quantity,booked_on,kind,reason,item,actor_id,received_id,source_line,acquisition_unit_cost,acquisition_correction) VALUES($1::text::bigint,$2::text::bigint,$3::text::numeric,'2026-10-01',$4,'Synthetic upgrade',$5,$6::text::bigint,$7::text::bigint,'0',$8::text::numeric,$9)")
   .bind(&c).bind(&item.id).bind(qty).bind(kind).bind(serde_json::to_value(&item.data).unwrap()).bind(&a).bind(if kind=="received" {Some(source.id.clone())}else{None}).bind(price).bind(correction).execute(&p).await.unwrap();
    }
    faktura_infrastructure::migrate(&p).await.unwrap();
    let r = PgStockReport(p)
        .report(
            &a,
            &c,
            Search {
                valuation: Valuation::Automatic,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.items[0].quantity, "6");
    assert_eq!(r.items[0].value.as_deref(), Some("180.00"));
    assert_eq!(r.items[0].issued_value.as_deref(), Some("120.00"));
    assert_eq!(r.items[0].correction_stock_value.as_deref(), Some("60.00"));
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn manual_acquisition_inventory_surplus_and_exact_reversals(p: PgPool) {
    use faktura_application::stock_report::{Search, StockReportStore, Valuation};
    use faktura_infrastructure::stock_report::PgStockReport;
    let (a, _, c, other) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Manual costs", &key()),
    )
    .await
    .unwrap();
    let store = PgStock(p.clone());
    let today = "2026-10-01".parse().unwrap();
    let mut opening = cmd(
        &item.id,
        stock::Action::Open {
            quantity: "10".into(),
            booked_on: today,
            reason: "Opening".into(),
        },
    );
    opening.acquisition_unit_cost = Some("20".into());
    // Exercise the flattened REST/MCP payload, including old payload compatibility.
    let payload = serde_json::to_value(&opening).unwrap();
    let first = store
        .execute(&a, &c, serde_json::from_value(payload.clone()).unwrap())
        .await
        .unwrap();
    assert_eq!(first.acquisition_unit_cost.as_deref(), Some("20"));
    assert_eq!(
        store
            .execute(&a, &c, serde_json::from_value(payload).unwrap())
            .await
            .unwrap()
            .id,
        first.id
    );
    store
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Move {
                    quantity: "-4".into(),
                    booked_on: today,
                    reason: "Issue".into(),
                },
            ),
        )
        .await
        .unwrap();
    let mut receipt = cmd(
        &item.id,
        stock::Action::Move {
            quantity: "5".into(),
            booked_on: today,
            reason: "Receipt".into(),
        },
    );
    receipt.acquisition_unit_cost = Some("40".into());
    let receipt = store.execute(&a, &c, receipt).await.unwrap();
    let revision = store.cards(&a, &c).await.unwrap()[0].revision;
    let mut count = cmd(
        &item.id,
        stock::Action::Count {
            quantity: "13".into(),
            revision,
            reason: "Surplus".into(),
        },
    );
    count.acquisition_unit_cost = Some("30".into());
    let count = store.execute(&a, &c, count).await.unwrap();
    let reports = PgStockReport(p.clone());
    let query = || Search {
        valuation: Valuation::Automatic,
        ..Default::default()
    };
    let r = reports.report(&a, &c, query()).await.unwrap();
    assert_eq!(r.known_value, "380.00");
    assert_eq!(r.items[0].issued_value.as_deref(), Some("80.00"));
    for (id, expected) in [(count.id, "320.00"), (receipt.id, "120.00")] {
        store
            .execute(
                &a,
                &c,
                cmd(
                    &item.id,
                    stock::Action::Reverse {
                        movement_id: id,
                        reason: "Undo".into(),
                    },
                ),
            )
            .await
            .unwrap();
        let r = reports.report(&a, &c, query()).await.unwrap();
        assert_eq!(r.known_value, expected);
        assert_eq!(r.items[0].issued_value.as_deref(), Some("80.00"));
    }
    for (quantity, cost) in [("-1", "10"), ("1", "0.00001"), ("1", "-1")] {
        let mut bad = cmd(
            &item.id,
            stock::Action::Move {
                quantity: quantity.into(),
                booked_on: today,
                reason: "Invalid".into(),
            },
        );
        bad.acquisition_unit_cost = Some(cost.into());
        assert!(store.execute(&a, &c, bad).await.is_err());
    }
    let mut free = cmd(
        &item.id,
        stock::Action::Move {
            quantity: "1".into(),
            booked_on: today,
            reason: "Free".into(),
        },
    );
    free.acquisition_unit_cost = Some("0".into());
    store.execute(&a, &c, free).await.unwrap();
    assert_eq!(
        reports.report(&a, &c, query()).await.unwrap().known_value,
        "120.00"
    );
    assert_eq!(reports.report(&a, &other, query()).await.unwrap().total, 0);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn item_sales_snapshots_discounts_credit_retail_exports_and_scope(p: PgPool) {
    use faktura_application::{
        credit_note, draft,
        export::{Format, Language},
        item_sales::{ExportRequest, ItemSalesStore, Search},
        retail::{self, RetailStore},
    };
    use faktura_infrastructure::{
        credit_note::PgCredit, draft::PgDraft, item_sales::PgItemSales, retail::PgRetail,
    };
    let (a, _, c, other) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("=Synthetic service", &key()),
    )
    .await
    .unwrap();
    let reports = PgItemSales(p.clone());
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.payment.method = "cash".into();
    data.supplier.registration_number = "12345678".into();
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    data.calculation.lines[0].vat_percent = "0".into();
    data.calculation.lines[0].catalog_id = Some(item.id.clone());
    data.calculation.lines[0].unit = item.data.unit.clone();
    data.calculation.lines[0].quantity = "2".into();
    data.calculation.lines[0].unit_price = "100".into();
    data.calculation.lines[0].discount_percent = "10".into();
    let mut free = data.calculation.lines[0].clone();
    free.id = "free".into();
    free.catalog_id = None;
    data.calculation.lines.push(free);
    let d = draft::create(
        &PgDraft(p.clone()),
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data: data.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        reports
            .report(&a, &c, Search::default())
            .await
            .unwrap()
            .total,
        0
    );
    let issued = draft::issue(
        &PgDraft(p.clone()),
        &a,
        &c,
        &d.id,
        draft::Issue {
            request_id: key(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    let r = reports.report(&a, &c, Search::default()).await.unwrap();
    assert_eq!(r.total, 1);
    assert_eq!(r.items[0].quantity, "2");
    assert_eq!(r.items[0].gross, "180.00");
    assert_eq!(r.totals[0].net, "180.00");
    // Current catalog price and archival must not alter stored sale amounts.
    sqlx::query("UPDATE catalog SET archived=true,data=jsonb_set(data,'{unit_price}','\"9999\"') WHERE id=$1").bind(item.id.parse::<i64>().unwrap()).execute(&p).await.unwrap();
    assert_eq!(
        reports
            .report(&a, &c, Search::default())
            .await
            .unwrap()
            .items[0]
            .gross,
        "180.00"
    );
    let credit = credit_note::create(
        &PgCredit(p.clone()),
        &a,
        &c,
        credit_note::Create {
            request_id: key(),
            invoice_id: issued.id,
        },
    )
    .await
    .unwrap();
    let mut correction = credit.data.clone();
    correction.reason = "Price correction".into();
    correction.lines.truncate(1);
    correction.lines[0].amount = "30".into();
    let credit = credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &credit.id,
        credit_note::Command {
            request_id: key(),
            revision: credit.revision,
            action: credit_note::Action::Update { data: correction },
        },
    )
    .await
    .unwrap();
    credit_note::command(
        &PgCredit(p.clone()),
        &a,
        &c,
        &credit.id,
        credit_note::Command {
            request_id: key(),
            revision: credit.revision,
            action: credit_note::Action::Issue,
        },
    )
    .await
    .unwrap();
    let r = reports
        .report(
            &a,
            &c,
            Search {
                catalog_id: Some(item.id.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.items[0].quantity, "2");
    assert_eq!(r.items[0].gross, "150.00");
    assert_eq!(r.source_total, 2);
    assert_eq!(
        reports
            .report(
                &a,
                &c,
                Search {
                    kind: Some("credit_note".into()),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items[0]
            .gross,
        "-30.00"
    );
    assert_eq!(
        reports
            .report(
                &a,
                &c,
                Search {
                    to: Some("2020-01-01".parse().unwrap()),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .total,
        0
    );
    assert_eq!(
        reports
            .report(&a, &other, Search::default())
            .await
            .unwrap()
            .total,
        0
    );
    assert!(reports
        .report("999999", &c, Search::default())
        .await
        .is_err());
    assert!(reports
        .report(
            &a,
            &c,
            Search {
                page: Some(0),
                ..Default::default()
            }
        )
        .await
        .is_err());
    assert!(reports
        .report(
            &a,
            &c,
            Search {
                from: Some("2026-10-20".parse().unwrap()),
                to: Some("2026-10-01".parse().unwrap()),
                ..Default::default()
            }
        )
        .await
        .is_err());
    sqlx::query("UPDATE catalog SET archived=false WHERE id=$1")
        .bind(item.id.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    data.calculation.lines.truncate(1);
    sqlx::query("UPDATE company SET settings=settings || $2 WHERE id=$1").bind(c.parse::<i64>().unwrap()).bind(serde_json::json!({"name":"Synthetic shop","registration_number":"12345678","street":"Test 1","city":"Praha","postal_code":"11000"})).execute(&p).await.unwrap();
    let retail = PgRetail(p.clone());
    let receipt = retail
        .execute(
            &a,
            &c,
            retail::Command {
                request_id: key(),
                action: retail::Action::Save {
                    id: None,
                    revision: None,
                    data: Box::new(retail::Data {
                        invoice: data,
                        terminal_id: None,
                        payments: faktura_domain::retail::Payments {
                            cash: "180".into(),
                            card: "0".into(),
                            voucher: "0".into(),
                        },
                        internal_note: String::new(),
                    }),
                    reason: "Sale".into(),
                },
            },
        )
        .await
        .unwrap();
    let r = reports.report(&a, &c, Search::default()).await.unwrap();
    assert_eq!(r.items[0].quantity, "4");
    assert_eq!(r.items[0].gross, "330.00");
    retail
        .execute(
            &a,
            &c,
            retail::Command {
                request_id: key(),
                action: retail::Action::Cancel {
                    id: receipt.id,
                    revision: receipt.revision,
                    reason: "Cancel".into(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(
        reports
            .report(&a, &c, Search::default())
            .await
            .unwrap()
            .items[0]
            .gross,
        "150.00"
    );
    for format in [Format::Csv, Format::Xlsx] {
        let file = reports
            .export(
                &a,
                &c,
                ExportRequest {
                    search: Search::default(),
                    format: format.clone(),
                    language: Language::En,
                },
            )
            .await
            .unwrap();
        match format {
            Format::Csv => {
                let csv = String::from_utf8(file.bytes).unwrap();
                assert!(csv.contains("150.00"));
                assert!(csv.contains("'=Synthetic service"));
            }
            Format::Xlsx => assert!(file.bytes.starts_with(b"PK")),
        }
    }
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn item_sales_pagination_units_totals_and_export_all_pages(p: PgPool) {
    use faktura_application::{
        draft,
        export::{Format, Language},
        item_sales::{ExportRequest, ItemSalesStore, Search},
    };
    use faktura_infrastructure::{draft::PgDraft, item_sales::PgItemSales};
    let (a, _, c, _) = setup(&p).await;
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.payment.method = "cash".into();
    data.supplier.registration_number = "12345678".into();
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    let mut line = data.calculation.lines[0].clone();
    line.quantity = "1".into();
    line.unit_price = "10".into();
    line.vat_percent = "0".into();
    line.unit = "ks".into();
    data.calculation.lines.clear();
    let mut first = String::new();
    for n in 0..27 {
        let item = catalog::create(
            &PgCatalog(p.clone()),
            &a,
            &c,
            create(&format!("Item {n:02}"), &key()),
        )
        .await
        .unwrap();
        if n == 0 {
            first = item.id.clone()
        }
        let mut l = line.clone();
        l.id = format!("line-{n}");
        l.catalog_id = Some(item.id);
        data.calculation.lines.push(l);
    }
    for n in 27..52 {
        let mut l = line.clone();
        l.id = format!("line-{n}");
        l.catalog_id = Some(first.clone());
        data.calculation.lines.push(l);
    }
    line.catalog_id = Some(first.clone());
    line.id = "other-unit".into();
    line.unit = "h".into();
    line.unit_price = "5".into();
    data.calculation.lines.push(line);
    let d = draft::create(
        &PgDraft(p.clone()),
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data,
        },
    )
    .await
    .unwrap();
    draft::issue(
        &PgDraft(p.clone()),
        &a,
        &c,
        &d.id,
        draft::Issue {
            request_id: key(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    let store = PgItemSales(p);
    let r = store.report(&a, &c, Search::default()).await.unwrap();
    assert_eq!(r.total, 28);
    assert_eq!(r.items.len(), 25);
    assert_eq!(r.totals[0].gross, "525.00");
    let r = store
        .report(
            &a,
            &c,
            Search {
                page: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.items.len(), 3);
    assert_eq!(r.totals[0].gross, "525.00");
    let r = store
        .report(
            &a,
            &c,
            Search {
                catalog_id: Some(first),
                source_page: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.total, 2);
    assert_eq!(r.source_total, 27);
    assert_eq!(r.sources.len(), 2);
    assert_eq!(r.totals[0].gross, "265.00");
    let file = store
        .export(
            &a,
            &c,
            ExportRequest {
                search: Search {
                    page: Some(2),
                    ..Default::default()
                },
                format: Format::Csv,
                language: Language::En,
            },
        )
        .await
        .unwrap();
    assert_eq!(String::from_utf8(file.bytes).unwrap().lines().count(), 29);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn foreign_purchase_physical_receipt_keeps_explicit_czk_valuation(p: PgPool) {
    use faktura_application::{
        received,
        stock_report::{Search, StockReportStore, Valuation},
    };
    use faktura_infrastructure::{received::PgReceived, stock_report::PgStockReport};
    let (a, _, c, other) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        create("Foreign purchased goods", &key()),
    )
    .await
    .unwrap();
    PgStock(p.clone())
        .execute(
            &a,
            &c,
            cmd(
                &item.id,
                stock::Action::Open {
                    quantity: "0".into(),
                    booked_on: "2026-10-01".parse().unwrap(),
                    reason: "Synthetic empty stock".into(),
                },
            ),
        )
        .await
        .unwrap();
    let mut purchase:faktura_domain::received::Data=serde_json::from_value(serde_json::json!({"supplier_contact_id":null,"supplier":{"name":"Foreign synthetic vendor","registration_number":"","vat_number":"","email":"","phone":"","address":{"street":"","city":"","postal_code":"","country":"DE"}},"external_number":"EUR-SYNTHETIC","variable_symbol":"","bank_account":"","issued_on":"2026-10-01","received_on":"2026-10-01","due_on":"2026-10-31","tax_on":null,"currency":"EUR","exchange_rate":{"rate":"25","units":"1"},"description":"Foreign purchase","note":"","rounding":"0","stock_incidental_cost":"10.00","lines":[{"description":"Goods","rate":"0","net":"100.00","vat":"0","stock":{"catalog_id":item.id,"quantity":"2","unit":item.data.unit,"acquisition_unit_cost":"20.00"}}]})).unwrap();
    let request = key();
    let store = PgReceived(p.clone());
    let doc = received::create(
        &store,
        &a,
        &c,
        received::Create {
            request_id: request.clone(),
            data: purchase.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(doc.totals.total, "100.00");
    assert_eq!(doc.data.currency, "EUR");
    assert_eq!(
        received::create(
            &store,
            &a,
            &c,
            received::Create {
                request_id: request,
                data: purchase.clone()
            }
        )
        .await
        .unwrap()
        .id,
        doc.id
    );
    let report = PgStockReport(p.clone())
        .report(
            &a,
            &c,
            Search {
                valuation: Valuation::Automatic,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(report.items[0].quantity, "2");
    assert_eq!(report.items[0].unit_cost.as_deref(), Some("25"));
    assert_eq!(report.known_value, "50.00");
    assert!(received::create(
        &store,
        &a,
        &other,
        received::Create {
            request_id: key(),
            data: purchase.clone()
        }
    )
    .await
    .is_err());
    purchase.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "30".into(),
        units: "1".into(),
    });
    purchase.external_number = "EUR-SYNTHETIC-2".into();
    purchase.stock_incidental_cost = None;
    purchase.lines[0]
        .stock
        .as_mut()
        .unwrap()
        .acquisition_unit_cost = None;
    received::create(
        &store,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: purchase,
        },
    )
    .await
    .unwrap();
    let report = PgStockReport(p.clone())
        .report(
            &a,
            &c,
            Search {
                valuation: Valuation::Automatic,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(report.items[0].quantity, "4");
    assert_eq!(report.items[0].unit_cost, None);
    assert_eq!(report.unvalued_count, 1);
}

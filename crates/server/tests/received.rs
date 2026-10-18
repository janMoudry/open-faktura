use faktura_application::{
    attachment::{self, AttachmentStore},
    auth, company,
    received::{self, Action, Command, Create, ReceivedStore, Search},
};
use faktura_domain::{
    company::CompanySettings,
    invoice::PartySnapshot,
    payment::PaymentInput,
    received::{Data, Line},
};
use faktura_infrastructure::{
    attachment::PgAttachment, auth::PgAuth, company::PgCompany, received::PgReceived,
};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn data() -> Data {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Default::default(),
        receipt_method: None,
        supplier_contact_id: None,
        supplier: serde_json::from_value::<PartySnapshot>(fixture["supplier"].clone()).unwrap(),
        external_number: "VENDOR-001".into(),
        variable_symbol: "000123".into(),
        bank_account: "123/0100".into(),
        issued_on: "2026-10-01".parse().unwrap(),
        received_on: "2026-10-13".parse().unwrap(),
        due_on: "2026-10-15".parse().unwrap(),
        tax_on: None,
        currency: "CZK".into(),
        description: "Synthetic expense".into(),
        note: "Private".into(),
        lines: vec![Line {
            stock: None,
            description: "Service".into(),
            rate: "21".into(),
            net: "100.01".into(),
            vat: "21.02".into(),
        }],
        rounding: "-0.03".into(),
    }
}
async fn setup(p: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        p.clone(),
        Some("synthetic-bootstrap-received-only-test".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-bootstrap-received-only-test",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, t) = auth::login(&auth, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let s = PgCompany(p.clone());
    let c = company::create(&s, &a.id, &key(), CompanySettings::default())
        .await
        .unwrap();
    let o = company::create(&s, &a.id, &key(), CompanySettings::default())
        .await
        .unwrap();
    (a.id, t, c.id, o.id)
}
async fn create(s: &PgReceived, a: &str, c: &str) -> received::Document {
    received::create(
        s,
        a,
        c,
        Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap()
}
async fn action(
    s: &PgReceived,
    a: &str,
    c: &str,
    d: &received::Document,
    action: Action,
) -> received::Document {
    received::command(
        s,
        a,
        c,
        &d.id,
        Command {
            request_id: key(),
            revision: d.revision,
            action,
        },
    )
    .await
    .unwrap()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn numbering_replay_source_totals_payments_and_reversals(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgReceived(p.clone());
    let k = key();
    let (x, y) = tokio::join!(
        received::create(
            &s,
            &a,
            &c,
            Create {
                request_id: k.clone(),
                data: data()
            }
        ),
        received::create(
            &s,
            &a,
            &c,
            Create {
                request_id: k.clone(),
                data: data()
            }
        )
    );
    let d = x.unwrap();
    assert_eq!(d.id, y.unwrap().id);
    assert_eq!(d.totals.total, "121.00");
    assert_eq!(d.number, "PF2026000001");
    let mut changed = data();
    changed.external_number = "different".into();
    assert!(received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: k,
            data: changed
        }
    )
    .await
    .is_err());
    let d = action(
        &s,
        &a,
        &c,
        &d,
        Action::Pay {
            payment: PaymentInput {
                amount: "21.00".into(),
                paid_on: "2026-10-13".parse().unwrap(),
                method: "bank_transfer".into(),
                note: "Paid".into(),
            },
        },
    )
    .await;
    assert_eq!(d.remaining, "100.00");
    assert_eq!(d.payment_status, "partial");
    assert!(received::command(
        &s,
        &a,
        &c,
        &d.id,
        Command {
            request_id: key(),
            revision: d.revision,
            action: Action::Update {
                data: Box::new(data())
            }
        }
    )
    .await
    .is_err());
    let d = action(
        &s,
        &a,
        &c,
        &d,
        Action::Reverse {
            payment_id: d.payments[0].id.clone(),
            reason: "Mistake".into(),
        },
    )
    .await;
    assert_eq!(d.remaining, "121.00");
    assert!(d.payments[0].reversed_at.is_some());
    let d = action(
        &s,
        &a,
        &c,
        &d,
        Action::Cancel {
            reason: "Duplicate".into(),
        },
    )
    .await;
    assert!(d.cancelled);
    assert_eq!(
        s.list(
            &a,
            &c,
            Search {
                status: "cancelled".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invoice_payment")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn scope_roles_revision_original_and_validation(p: PgPool) {
    use base64::Engine;
    let (a, _, c, o) = setup(&p).await;
    let s = PgReceived(p.clone());
    let d = create(&s, &a, &c).await;
    assert!(s.get(&a, &o, &d.id).await.is_err());
    let mut bad = data();
    bad.supplier_contact_id = Some("999999".into());
    assert!(received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: key(),
            data: bad
        }
    )
    .await
    .is_err());
    let files = PgAttachment(p.clone());
    let bytes = b"%PDF-1.4\nsynthetic source";
    let f = attachment::upload(
        &files,
        &a,
        &c,
        "received",
        &d.id,
        attachment::Upload {
            request_id: key(),
            filename: "source.pdf".into(),
            base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        files
            .download(&a, &c, "received", &d.id, &f.id, None)
            .await
            .unwrap()
            .content,
        bytes
    );
    assert!(files
        .download(&a, &o, "received", &d.id, &f.id, None)
        .await
        .is_err());
    let mut bad = data();
    bad.lines[0].vat = "0.001".into();
    assert!(s.preview(&a, &c, bad).await.is_err());
    let newer = action(
        &s,
        &a,
        &c,
        &d,
        Action::Update {
            data: Box::new(data()),
        },
    )
    .await;
    assert!(received::command(
        &s,
        &a,
        &c,
        &d.id,
        Command {
            request_id: key(),
            revision: d.revision,
            action: Action::Cancel {
                reason: "stale".into()
            }
        }
    )
    .await
    .is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    assert!(s.get(&a, &c, &d.id).await.is_ok());
    assert!(received::command(
        &s,
        &a,
        &c,
        &d.id,
        Command {
            request_id: key(),
            revision: newer.revision,
            action: Action::Cancel {
                reason: "No permission".into()
            }
        }
    )
    .await
    .is_err());
    assert!(files
        .delete(&a, &c, "received", &d.id, &f.id)
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_payments_cannot_overpay(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgReceived(p.clone());
    let d = create(&s, &a, &c).await;
    let cmd = || Command {
        request_id: key(),
        revision: d.revision,
        action: Action::Pay {
            payment: PaymentInput {
                amount: "121.00".into(),
                paid_on: "2026-10-13".parse().unwrap(),
                method: "cash".into(),
                note: "".into(),
            },
        },
    };
    let (x, y) = tokio::join!(
        received::command(&s, &a, &c, &d.id, cmd()),
        received::command(&s, &a, &c, &d.id, cmd())
    );
    assert_ne!(x.is_ok(), y.is_ok());
    let d = s.get(&a, &c, &d.id).await.unwrap();
    assert_eq!(d.payment_status, "paid");
    assert_eq!(d.remaining, "0.00");
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL REST MCP"]
async fn rest_mcp_contract_and_origin(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let s = PgReceived(p.clone());
    let d = create(&s, &a, &c).await;
    let app =
        faktura_server::received::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/received-invoices/{}", d.id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
    assert_eq!(http["totals"]["total"], "121.00");
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/received-invoices"))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .header("origin", "https://foreign.example.test")
                .body(Body::from(
                    serde_json::to_vec(&Create {
                        request_id: key(),
                        data: data(),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
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
            CallToolRequestParams::new("received_get").with_arguments(
                serde_json::json!({"company_id":c,"id":d.id})
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
    assert_eq!(mcp, http);
    let created = client
        .call_tool(
            CallToolRequestParams::new("received_create").with_arguments(
                serde_json::json!({"company_id":c,"request_id":key(),"data":receipt()})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(created.is_error, Some(true));
    let created_json = serde_json::to_value(created).unwrap();
    let created_doc: serde_json::Value =
        serde_json::from_str(created_json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(created_doc["paid"], "121.00");
    for (path, tool) in [
        ("expense-payments", "received_expenses"),
        ("expense-overview", "received_overview"),
    ] {
        let r = faktura_server::received::router(
            p.clone(),
            vec!["http://localhost:5173".into()],
            false,
        )
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/received-invoices/{path}?from=2026-10-01&to=2026-10-31"
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(r.status(), 200);
        let http: serde_json::Value =
            serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
        let result = client
            .call_tool(
                CallToolRequestParams::new(tool).with_arguments(
                    serde_json::json!({"company_id":c,"from":"2026-10-01","to":"2026-10-31"})
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
    }
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_024_preserves_existing_documents(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610130025)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    use base64::Engine;
    use faktura_application::draft::{self, DraftStore};
    use faktura_infrastructure::draft::PgDraft;
    let mut draft_data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    draft_data.supplier.registration_number = "12345678".into();
    draft_data.supplier.email = "reply@example.test".into();
    draft_data.payment.method = "cash".into();
    draft_data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    draft_data.calculation.lines[0].vat_percent = "0".into();
    draft_data.tax_on = None;
    let store = PgDraft(p.clone());
    let invoice = draft::create(
        &store,
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data: draft_data,
        },
    )
    .await
    .unwrap();
    let invoice = draft::issue(
        &store,
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
    let pdf = store.pdf(&a, &c, &invoice.id).await.unwrap();
    let files = PgAttachment(p.clone());
    let f = attachment::upload(
        &files,
        &a,
        &c,
        "invoice",
        &invoice.id,
        attachment::Upload {
            request_id: key(),
            filename: "old.txt".into(),
            base64: base64::engine::general_purpose::STANDARD.encode(b"Old source"),
        },
    )
    .await
    .unwrap();
    let before = company::CompanyStore::get(&PgCompany(p.clone()), &a, &c)
        .await
        .unwrap();
    faktura_infrastructure::migrate(&p).await.unwrap();
    assert_eq!(store.pdf(&a, &c, &invoice.id).await.unwrap(), pdf);
    assert_eq!(
        files
            .download(&a, &c, "invoice", &invoice.id, &f.id, None)
            .await
            .unwrap()
            .content,
        b"Old source"
    );

    let after = company::CompanyStore::get(&PgCompany(p.clone()), &a, &c)
        .await
        .unwrap();
    assert_eq!(before.revision, after.revision);
    let d = create(&PgReceived(p.clone()), &a, &c).await;
    assert_eq!(d.number, "PF2026000001");
}

fn receipt() -> Data {
    let mut d = data();
    d.kind = faktura_domain::received::Kind::Receipt;
    d.receipt_method = Some("card".into());
    d.supplier.name.clear();
    d.external_number.clear();
    d.variable_symbol.clear();
    d.bank_account.clear();
    d.received_on = d.issued_on;
    d.due_on = d.issued_on;
    d
}
fn period() -> faktura_application::purchase_report::Period {
    faktura_application::purchase_report::Period {
        from: "2026-10-01".parse().unwrap(),
        to: "2026-10-31".parse().unwrap(),
    }
}
fn expense_search() -> faktura_application::purchase_report::Search {
    faktura_application::purchase_report::Search {
        from: period().from,
        to: period().to,
        query: String::new(),
        method: String::new(),
        status: String::new(),
        page: None,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn receipt_atomic_replay_revision_edit_and_cancel(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgReceived(p.clone());
    let k = key();
    let cmd = || Create {
        request_id: k.clone(),
        data: receipt(),
    };
    let (one, two) = tokio::join!(
        received::create(&s, &a, &c, cmd()),
        received::create(&s, &a, &c, cmd())
    );
    let d = one.unwrap();
    assert_eq!(d.id, two.unwrap().id);
    assert_eq!(d.number, "UC2026000001");
    assert_eq!(d.paid, "121.00");
    assert_eq!(d.remaining, "0.00");
    assert_eq!(d.payments.len(), 1);
    assert_eq!(d.payments[0].input.method, "card");
    assert_eq!(create(&s, &a, &c).await.number, "PF2026000001");
    let invoice_list = s.list(&a, &c, Search::default()).await.unwrap();
    assert_eq!(invoice_list.total, 1);
    assert_eq!(
        s.list(
            &a,
            &c,
            Search {
                kind: faktura_domain::received::Kind::Receipt,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    let mut changed = d.data.clone();
    changed.lines[0].net = "200.01".into();
    changed.receipt_method = Some("cash".into());
    let k = key();
    let cmd = || Command {
        request_id: k.clone(),
        revision: d.revision,
        action: Action::Update {
            data: Box::new(changed.clone()),
        },
    };
    let updated = received::command(&s, &a, &c, &d.id, cmd()).await.unwrap();
    assert_eq!(
        received::command(&s, &a, &c, &d.id, cmd())
            .await
            .unwrap()
            .revision,
        updated.revision
    );
    assert_eq!(updated.paid, "221.00");
    assert_eq!(updated.payments.len(), 2);
    assert!(updated.payments[0].reversed_at.is_some());
    assert_eq!(updated.payments[1].input.method, "cash");
    assert!(received::command(
        &s,
        &a,
        &c,
        &d.id,
        Command {
            request_id: key(),
            revision: d.revision,
            action: Action::Cancel {
                reason: "stale".into()
            }
        }
    )
    .await
    .is_err());
    assert!(received::command(
        &s,
        &a,
        &c,
        &d.id,
        Command {
            request_id: key(),
            revision: updated.revision,
            action: Action::Reverse {
                payment_id: updated.payments[1].id.clone(),
                reason: "individual reversal".into()
            }
        }
    )
    .await
    .is_err());
    let cancelled = action(
        &s,
        &a,
        &c,
        &updated,
        Action::Cancel {
            reason: "Duplicate purchase".into(),
        },
    )
    .await;
    assert_eq!(cancelled.paid, "0.00");
    assert_eq!(cancelled.remaining, "0.00");
    assert!(cancelled.payments.iter().all(|p| p.reversed_at.is_some()));
    assert_eq!(
        cancelled.payments[1].reason.as_deref(),
        Some("Duplicate purchase")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invoice_payment")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn expense_totals_periods_filters_and_isolation(p: PgPool) {
    use faktura_application::purchase_report::PurchaseReportStore;
    use faktura_infrastructure::purchase_report::PgPurchaseReport;
    let (a, _, c, other) = setup(&p).await;
    let s = PgReceived(p.clone());
    let report = PgPurchaseReport(p.clone());
    let invoice = create(&s, &a, &c).await;
    let invoice = action(
        &s,
        &a,
        &c,
        &invoice,
        Action::Pay {
            payment: PaymentInput {
                amount: "21.00".into(),
                paid_on: period().from,
                method: "bank_transfer".into(),
                note: "Deposit reference".into(),
            },
        },
    )
    .await;
    let r = received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: key(),
            data: receipt(),
        },
    )
    .await
    .unwrap();
    let total = report.overview(&a, &c, period()).await.unwrap();
    assert_eq!(total.purchases, "242.00");
    assert_eq!(total.expenses, "142.00");
    assert_eq!(total.outstanding, "100.00");
    let empty = report
        .overview(
            &a,
            &c,
            faktura_application::purchase_report::Period {
                from: "2026-11-01".parse().unwrap(),
                to: "2026-11-30".parse().unwrap(),
            },
        )
        .await
        .unwrap();
    assert_eq!(empty.purchases, "0.00");
    assert_eq!(empty.expenses, "0.00");
    assert_eq!(empty.outstanding, "100.00");
    let mut q = expense_search();
    q.method = "card".into();
    let list = report.payments(&a, &c, q).await.unwrap();
    assert_eq!(list.total, 1);
    assert_eq!(list.items[0].document_id, r.id);
    let mut q = expense_search();
    q.query = "deposit reference".into();
    assert_eq!(report.payments(&a, &c, q).await.unwrap().total, 1);
    let other_total = report.overview(&a, &other, period()).await.unwrap();
    assert_eq!(other_total.expenses, "0.00");
    assert_eq!(
        report
            .payments(&a, &other, expense_search())
            .await
            .unwrap()
            .total,
        0
    );
    action(
        &s,
        &a,
        &c,
        &r,
        Action::Cancel {
            reason: "Duplicate".into(),
        },
    )
    .await;
    let total = report.overview(&a, &c, period()).await.unwrap();
    assert_eq!(total.purchases, "121.00");
    assert_eq!(total.expenses, "21.00");
    let mut q = expense_search();
    q.status = "reversed".into();
    assert_eq!(report.payments(&a, &c, q).await.unwrap().total, 1);
    action(
        &s,
        &a,
        &c,
        &invoice,
        Action::Reverse {
            payment_id: invoice.payments[0].id.clone(),
            reason: "Correction".into(),
        },
    )
    .await;
    let total = report.overview(&a, &c, period()).await.unwrap();
    assert_eq!(total.expenses, "0.00");
    assert_eq!(total.outstanding, "121.00");
    let mut q = expense_search();
    q.to = "2026-09-01".parse().unwrap();
    assert!(report.payments(&a, &c, q).await.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    assert!(report.overview(&a, &c, period()).await.is_ok());
    assert!(received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: key(),
            data: receipt()
        }
    )
    .await
    .is_err());
    sqlx::query("DELETE FROM membership WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    assert!(report.overview(&a, &c, period()).await.is_err());
    assert!(report.payments(&a, &c, expense_search()).await.is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn receipt_validation_pagination_and_aggregate_all_pages(p: PgPool) {
    use faktura_application::purchase_report::PurchaseReportStore;
    use faktura_infrastructure::purchase_report::PgPurchaseReport;
    let (a, _, c, _) = setup(&p).await;
    let s = PgReceived(p.clone());
    let report = PgPurchaseReport(p.clone());
    for _ in 0..26 {
        received::create(
            &s,
            &a,
            &c,
            Create {
                request_id: key(),
                data: receipt(),
            },
        )
        .await
        .unwrap();
    }
    let first = report.payments(&a, &c, expense_search()).await.unwrap();
    assert_eq!(first.total, 26);
    assert_eq!(first.items.len(), 25);
    let mut q = expense_search();
    q.page = Some(2);
    let second = report.payments(&a, &c, q).await.unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(!first
        .items
        .iter()
        .any(|i| i.document_id == second.items[0].document_id));
    let total = report.overview(&a, &c, period()).await.unwrap();
    assert_eq!(total.purchases, "3146.00");
    assert_eq!(total.expenses, "3146.00");
    assert_eq!(total.outstanding, "0.00");
    let mut old = data();
    old.issued_on = "2000-01-01".parse().unwrap();
    old.received_on = old.issued_on;
    old.due_on = old.issued_on;
    received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: key(),
            data: old,
        },
    )
    .await
    .unwrap();
    let total = report.overview(&a, &c, period()).await.unwrap();
    assert_eq!(total.purchases, "3146.00");
    assert_eq!(total.outstanding, "121.00");
    assert_eq!(total.overdue, "121.00");
    for field in ["method", "dates", "precision", "kind"] {
        let mut d = receipt();
        match field {
            "method" => d.receipt_method = Some("bank_transfer".into()),
            "dates" => d.due_on = "2026-10-02".parse().unwrap(),
            "precision" => d.lines[0].vat = "0.001".into(),
            _ => d.kind = Default::default(),
        }
        assert!(received::create(
            &s,
            &a,
            &c,
            Create {
                request_id: key(),
                data: d
            }
        )
        .await
        .is_err());
    }
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_025_keeps_old_invoice_replay_and_original(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610130026)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let k = key();
    let d = data();
    let json = serde_json::to_value(&d).unwrap();
    assert!(json.get("kind").is_none());
    let id:i64=sqlx::query_scalar("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) VALUES($1,'PF2026000001',$2,121,'2026-10-13','2026-10-15') RETURNING id").bind(c.parse::<i64>().unwrap()).bind(json).fetch_one(&p).await.unwrap();
    sqlx::query("INSERT INTO received_series(company_id,year,next_value) VALUES($1,2026,2)")
        .bind(c.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    let s = PgReceived(p.clone());
    let old = s.get(&a, &c, &id.to_string()).await.unwrap();
    let cmd = Create {
        request_id: k.clone(),
        data: d,
    };
    sqlx::query("INSERT INTO received_command(company_id,request_id,actor_id,invoice_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5,$6)")
      .bind(c.parse::<i64>().unwrap()).bind(&k).bind(a.parse::<i64>().unwrap()).bind(id)
      .bind(serde_json::json!({"create":&cmd})).bind(serde_json::to_value(&old).unwrap()).execute(&p).await.unwrap();
    use base64::Engine;
    let files = PgAttachment(p.clone());
    let f = attachment::upload(
        &files,
        &a,
        &c,
        "received",
        &old.id,
        attachment::Upload {
            request_id: key(),
            filename: "old.txt".into(),
            base64: base64::engine::general_purpose::STANDARD.encode(b"Old original"),
        },
    )
    .await
    .unwrap();
    faktura_infrastructure::migrate(&p).await.unwrap();
    assert_eq!(received::create(&s, &a, &c, cmd).await.unwrap().id, old.id);
    assert_eq!(create(&s, &a, &c).await.number, "PF2026000002");
    let r = received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: key(),
            data: receipt(),
        },
    )
    .await
    .unwrap();
    assert_eq!(r.number, "UC2026000001");
    assert_eq!(
        files
            .download(&a, &c, "received", &old.id, &f.id, None)
            .await
            .unwrap()
            .content,
        b"Old original"
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_purchase_preserves_currency_and_converts_reports(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let mut d = data();
    d.currency = "JPY".into();
    d.rounding = "0.00".into();
    d.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "15.125".into(),
        units: "100".into(),
    });
    d.lines = vec![Line {
        stock: None,
        description: "Synthetic foreign purchase".into(),
        rate: "21".into(),
        net: "1000.00".into(),
        vat: "210.00".into(),
    }];
    let s = PgReceived(p.clone());
    let d = received::create(
        &s,
        &a,
        &c,
        Create {
            request_id: key(),
            data: d,
        },
    )
    .await
    .unwrap();
    assert_eq!(d.totals.total, "1210.00");
    let d = action(
        &s,
        &a,
        &c,
        &d,
        Action::Pay {
            payment: PaymentInput {
                amount: "400.00".into(),
                paid_on: "2026-10-13".parse().unwrap(),
                method: "bank_transfer".into(),
                note: String::new(),
            },
        },
    )
    .await;
    assert_eq!(d.remaining, "810.00");
    use faktura_application::dashboard::DashboardStore;
    let r = faktura_infrastructure::dashboard::PgDashboard(p.clone())
        .dashboard(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(r.purchases.gross, "183.01");
    assert_eq!(r.purchases.outstanding, "122.51");
    assert_eq!(r.purchases.settled, "60.50");
    use faktura_application::tax_report::TaxReportStore;
    let tax = faktura_infrastructure::tax_report::PgTaxReport(p.clone())
        .report(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(tax.input, "31.76");
}

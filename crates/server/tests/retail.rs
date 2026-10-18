use faktura_application::{
    auth,
    cash::{self, CashStore},
    company,
    retail::{self, RetailStore},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, cash::PgCash, company::PgCompany, retail::PgRetail};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
async fn setup(p: &PgPool) -> (String, String, String, String) {
    let s = PgAuth::new(p.clone(), Some("synthetic-retail-bootstrap-only".into()));
    auth::bootstrap(
        &s,
        "synthetic-retail-bootstrap-only",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, t) = auth::login(&s, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let settings = CompanySettings {
        name: "Synthetic shop".into(),
        registration_number: "12345678".into(),
        street: "Test 1".into(),
        city: "Praha".into(),
        postal_code: "11000".into(),
        ..Default::default()
    };
    let c = company::create(&PgCompany(p.clone()), &a.id, &key(), settings.clone())
        .await
        .unwrap();
    let other = company::create(&PgCompany(p.clone()), &a.id, &key(), settings)
        .await
        .unwrap();
    (a.id, t, c.id, other.id)
}
fn data() -> retail::Data {
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.calculation.lines[0].unit_price = "100".into();
    invoice.calculation.lines[0].quantity = "1".into();
    invoice.tax_on = None;
    invoice.customer = None;
    retail::Data {
        invoice,
        terminal_id: None,
        payments: faktura_domain::retail::Payments {
            cash: "50".into(),
            card: "30".into(),
            voucher: "20".into(),
        },
        internal_note: "Private test note".into(),
    }
}
fn create(d: retail::Data) -> retail::Command {
    retail::Command {
        request_id: key(),
        action: retail::Action::Save {
            id: None,
            revision: None,
            data: Box::new(d),
            reason: "Synthetic sale".into(),
        },
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn exact_payments_replay_scope_pdf_revision_and_cancel(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgRetail(p.clone());
    let cmd = create(data());
    let serialized = serde_json::to_value(&cmd).unwrap();
    let d = s.execute(&a, &c, cmd).await.unwrap();
    assert_eq!(d.totals.payable, "100.00");
    assert_eq!(d.allocations.len(), 3);
    assert_eq!(d.data.invoice.supplier.name, "Synthetic shop");
    assert_eq!(d.number, "PR20260001");
    assert!(d.data.invoice.customer.is_none());
    assert_eq!(
        s.execute(&a, &c, serde_json::from_value(serialized).unwrap())
            .await
            .unwrap()
            .id,
        d.id
    );
    assert!(s.get(&a, &other, &d.id).await.is_err());
    assert!(s.pdf(&a, &other, &d.id).await.is_err());
    let pdf = s.pdf(&a, &c, &d.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert_eq!(pdf, s.pdf(&a, &c, &d.id).await.unwrap());
    let mut bad = data();
    bad.payments.cash = "51".into();
    assert!(s.execute(&a, &c, create(bad)).await.is_err());
    let next = s.execute(&a, &c, create(data())).await.unwrap();
    assert_eq!(next.number, "PR20260002");
    let cancel = retail::Command {
        request_id: key(),
        action: retail::Action::Cancel {
            id: d.id.clone(),
            revision: d.revision,
            reason: "Returned sale".into(),
        },
    };
    let d = s.execute(&a, &c, cancel).await.unwrap();
    assert!(d.cancelled);
    assert_ne!(pdf, s.pdf(&a, &c, &d.id).await.unwrap());
    assert_eq!(s.history(&a, &c, &d.id).await.unwrap().len(), 2);
    assert!(s
        .execute(
            &a,
            &c,
            retail::Command {
                request_id: key(),
                action: retail::Action::Save {
                    id: Some(d.id.clone()),
                    revision: Some(d.revision),
                    data: Box::new(data()),
                    reason: "Cannot edit cancellation".into()
                }
            }
        )
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn terminal_snapshot_concurrent_numbering_and_viewer(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let cash = PgCash(p.clone());
    let cash::Reply::Terminal(t) = cash
        .execute(
            &a,
            &c,
            cash::Command {
                request_id: key(),
                action: cash::Action::Terminal {
                    id: None,
                    revision: None,
                    code: "FRONT".into(),
                    name: "Original desk".into(),
                    archived: false,
                },
            },
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let s = PgRetail(p.clone());
    let mut input = data();
    input.terminal_id = Some(t.id.clone());
    assert!(s.execute(&a, &other, create(input.clone())).await.is_err());
    let cmd = create(input.clone());
    let serialized = serde_json::to_value(&cmd).unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, cmd),
        s.execute(&a, &c, serde_json::from_value(serialized).unwrap())
    );
    let one = one.unwrap();
    assert_eq!(one.id, two.unwrap().id);
    cash.execute(
        &a,
        &c,
        cash::Command {
            request_id: key(),
            action: cash::Action::Terminal {
                id: Some(t.id),
                revision: Some(t.revision),
                code: "FRONT".into(),
                name: "Changed".into(),
                archived: true,
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(
        s.get(&a, &c, &one.id).await.unwrap().terminal.unwrap().name,
        "Original desk"
    );
    assert!(s.execute(&a, &c, create(input)).await.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.get(&a, &c, &one.id).await.is_ok());
    assert!(s.execute(&a, &c, create(data())).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn posting_exact_groups_replay_reversal_and_source_guards(p: PgPool) {
    use retail::PostingStore;
    let (a, _, c, other) = setup(&p).await;
    let s = PgRetail(p.clone());
    let cash = PgCash(p.clone());
    let cash::Reply::Account(account) = cash
        .execute(
            &a,
            &c,
            cash::Command {
                request_id: key(),
                action: cash::Action::Account {
                    id: None,
                    revision: None,
                    data: cash::AccountData {
                        name: "Shop".into(),
                        currency: "CZK".into(),
                        opening_on: "2026-01-01".parse().unwrap(),
                        opening_balance: "0".into(),
                    },
                    archived: false,
                },
            },
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let one = s.execute(&a, &c, create(data())).await.unwrap();
    let two = s.execute(&a, &c, create(data())).await.unwrap();
    let input = retail::PostingInput {
        document_ids: vec![one.id.clone(), two.id.clone()],
        booked_on: "2026-10-15".parse().unwrap(),
        cash_account_id: Some(account.id.clone()),
    };
    assert!(s.posting_preview(&a, &other, input.clone()).await.is_err());
    let preview = s.posting_preview(&a, &c, input.clone()).await.unwrap();
    assert_eq!(preview.groups.len(), 3);
    assert_eq!(
        preview
            .groups
            .iter()
            .find(|v| v.method == "cash")
            .unwrap()
            .paid,
        "100.00"
    );
    assert_eq!(
        preview
            .groups
            .iter()
            .find(|v| v.method == "card")
            .unwrap()
            .paid,
        "60.00"
    );
    let cmd = retail::PostCommand {
        request_id: key(),
        input: input.clone(),
        fingerprint: preview.fingerprint,
    };
    let serialized = serde_json::to_value(&cmd).unwrap();
    let posted = s.post(&a, &c, cmd).await.unwrap();
    assert_eq!(
        s.post(&a, &c, serde_json::from_value(serialized).unwrap())
            .await
            .unwrap()
            .id,
        posted.id
    );
    assert!(s.posting_preview(&a, &c, input.clone()).await.is_err());
    let source = s.get(&a, &c, &one.id).await.unwrap();
    assert_eq!(source.posting_id, Some(posted.id.clone()));
    assert!(s
        .execute(
            &a,
            &c,
            retail::Command {
                request_id: key(),
                action: retail::Action::Cancel {
                    id: source.id,
                    revision: source.revision,
                    reason: "Blocked".into()
                }
            }
        )
        .await
        .is_err());
    let movement = posted
        .groups
        .iter()
        .find_map(|g| g.cash_movement_id.clone())
        .unwrap();
    assert!(cash
        .execute(
            &a,
            &c,
            cash::Command {
                request_id: key(),
                action: cash::Action::Cancel {
                    movement_id: movement,
                    revision: 1,
                    reason: "Must reverse batch".into()
                }
            }
        )
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cash_movement WHERE company_id=$1::text::bigint AND NOT cancelled",
    )
    .bind(&c)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let reverse = retail::UnpostCommand {
        request_id: key(),
        id: posted.id,
        revision: posted.revision,
        reason: "Correction".into(),
    };
    assert!(s.unpost(&a, &c, reverse).await.unwrap().cancelled);
    assert!(s.get(&a, &c, &one.id).await.unwrap().posting_id.is_none());
    assert!(s.posting_preview(&a, &c, input).await.is_ok());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cash_movement WHERE company_id=$1::text::bigint AND NOT cancelled",
    )
    .bind(&c)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(count, 0);
    assert_eq!(s.history(&a, &c, &one.id).await.unwrap().len(), 3);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn posting_stale_preview_concurrency_and_mcp_parity(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use retail::PostingStore;
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let s = PgRetail(p.clone());
    let mut d = data();
    d.payments.cash = "0".into();
    d.payments.card = "80".into();
    let doc = s.execute(&a, &c, create(d.clone())).await.unwrap();
    let input = retail::PostingInput {
        document_ids: vec![doc.id.clone()],
        booked_on: "2026-10-15".parse().unwrap(),
        cash_account_id: None,
    };
    let before = s.posting_preview(&a, &c, input.clone()).await.unwrap();
    s.execute(
        &a,
        &c,
        retail::Command {
            request_id: key(),
            action: retail::Action::Save {
                id: Some(doc.id.clone()),
                revision: Some(doc.revision),
                data: Box::new(d),
                reason: "Changed".into(),
            },
        },
    )
    .await
    .unwrap();
    assert!(s
        .post(
            &a,
            &c,
            retail::PostCommand {
                request_id: key(),
                input: input.clone(),
                fingerprint: before.fingerprint
            }
        )
        .await
        .is_err());
    assert!(s.postings(&a, &c).await.unwrap().is_empty());
    let preview = s.posting_preview(&a, &c, input.clone()).await.unwrap();
    let app =
        faktura_server::retail::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let http = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/retail/postings/preview"))
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("content-type", "application/json")
                .header("x-faktura-request", "1")
                .body(Body::from(serde_json::to_vec(&input).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(http.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(http.into_body(), 1000000).await.unwrap()).unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(p.clone(), token);
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
    let mut args = serde_json::to_value(&input).unwrap();
    args["company_id"] = c.clone().into();
    let reply = client
        .call_tool(
            CallToolRequestParams::new("retail_posting_preview")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(reply.is_error, Some(true));
    let json = serde_json::to_value(reply).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http, mcp);
    let left = retail::PostCommand {
        request_id: key(),
        input: input.clone(),
        fingerprint: preview.fingerprint.clone(),
    };
    let right = retail::PostCommand {
        request_id: key(),
        input,
        fingerprint: preview.fingerprint,
    };
    let (left, right) = tokio::join!(s.post(&a, &c, left), s.post(&a, &c, right));
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert_eq!(s.postings(&a, &c).await.unwrap().len(), 1);
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn stock_sale_edit_and_cancel_are_exact(p: PgPool) {
    use faktura_application::{
        catalog,
        stock::{Action, Command, StockStore},
    };
    use faktura_infrastructure::{catalog::PgCatalog, stock::PgStock};
    let (a, _, c, _) = setup(&p).await;
    let item = catalog::create(
        &PgCatalog(p.clone()),
        &a,
        &c,
        catalog::Create {
            request_id: key(),
            data: faktura_domain::catalog::CatalogData {
                name: "Goods".into(),
                ..Default::default()
            },
        },
    )
    .await
    .unwrap();
    let stock = PgStock(p.clone());
    stock
        .execute(
            &a,
            &c,
            Command {
                acquisition_unit_cost: None,
                request_id: key(),
                catalog_id: item.id.clone(),
                action: Action::Open {
                    quantity: "5".into(),
                    booked_on: "2026-10-01".parse().unwrap(),
                    reason: "Initial".into(),
                },
            },
        )
        .await
        .unwrap();
    let mut data = data();
    data.invoice.calculation.lines[0].catalog_id = Some(item.id);
    data.invoice.calculation.lines[0].unit = item.data.unit;
    let store = PgRetail(p.clone());
    let sale = store.execute(&a, &c, create(data.clone())).await.unwrap();
    assert_eq!(stock.cards(&a, &c).await.unwrap()[0].quantity, "4");
    data.invoice.calculation.lines[0].quantity = "2".into();
    data.payments.cash = "150".into();
    let sale = store
        .execute(
            &a,
            &c,
            retail::Command {
                request_id: key(),
                action: retail::Action::Save {
                    id: Some(sale.id),
                    revision: Some(sale.revision),
                    data: Box::new(data),
                    reason: "Two units".into(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(stock.cards(&a, &c).await.unwrap()[0].quantity, "3");
    store
        .execute(
            &a,
            &c,
            retail::Command {
                request_id: key(),
                action: retail::Action::Cancel {
                    id: sale.id,
                    revision: sale.revision,
                    reason: "Returned goods".into(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(stock.cards(&a, &c).await.unwrap()[0].quantity, "5");
}

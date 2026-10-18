use faktura_application::{
    auth,
    banking::{self, Account, Action, BankStore, Candidate, Command, Movement, Reply},
    company, draft,
    payment::{self, PaymentStore},
    received::{self, ReceivedStore},
};
use faktura_domain::{
    banking::{AccountData, MovementData},
    company::CompanySettings,
    invoice::{DraftData, PartySnapshot, TaxMode},
    payment::PaymentInput,
    received::{Data, Line},
};
use faktura_infrastructure::{
    auth::PgAuth, banking::PgBank, company::PgCompany, draft::PgDraft, payment::PgPayment,
    received::PgReceived,
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
async fn issued_case(
    pool: &PgPool,
    actor: &str,
    company: &str,
    days: i32,
    zero: bool,
) -> draft::Draft {
    let mut d: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    let today: String =
        sqlx::query_scalar("SELECT ((now() AT TIME ZONE 'Europe/Prague')::date-$1)::text")
            .bind(days)
            .fetch_one(pool)
            .await
            .unwrap();
    d.issued_on = Some(today.parse().unwrap());
    d.due_on = d.issued_on;
    d.tax_on = None;
    d.supplier.registration_number = "12345678".into();
    d.payment.method = "cash".into();
    d.calculation.tax_mode = TaxMode::NonPayer;
    d.calculation.lines[0].vat_percent = "0".into();
    d.calculation.lines[0].unit_price = "100".into();
    if zero {
        d.calculation.lines[0].unit_price = "0".into();
    }
    let year = d.issued_on.unwrap().format("%Y").to_string();
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{number_year}',$2::text::jsonb) WHERE id=$1::text::bigint").bind(company).bind(year).execute(pool).await.unwrap();
    let store = PgDraft(pool.clone());
    let draft = draft::create(
        &store,
        actor,
        company,
        draft::Create {
            request_id: key(),
            data: d,
        },
    )
    .await
    .unwrap();
    draft::issue(
        &store,
        actor,
        company,
        &draft.id,
        draft::Issue {
            request_id: key(),
            revision: 1,
        },
    )
    .await
    .unwrap()
}
async fn run(s: &PgBank, a: &str, c: &str, action: Action) -> Reply {
    s.execute(
        a,
        c,
        Command {
            request_id: key(),
            action,
        },
    )
    .await
    .unwrap()
}
async fn account(s: &PgBank, a: &str, c: &str) -> Account {
    match run(
        s,
        a,
        c,
        Action::Account {
            id: None,
            revision: None,
            data: AccountData {
                name: "Operating".into(),
                iban: "CZ6508000000192000145399".into(),
                currency: "CZK".into(),
            },
            archived: false,
        },
    )
    .await
    {
        Reply::Account(v) => v,
        _ => panic!(),
    }
}
async fn movement(s: &PgBank, a: &str, c: &str, account: &Account, amount: &str) -> Movement {
    match run(
        s,
        a,
        c,
        Action::Manual {
            account_id: account.id.clone(),
            data: MovementData {
                external_id: "".into(),
                booked_on: "2026-10-01".parse().unwrap(),
                amount: amount.into(),
                currency: account.data.currency.clone(),
                variable_symbol: "000123".into(),
                counterparty: "Synthetic vendor".into(),
                description: "Bank test".into(),
            },
        },
    )
    .await
    {
        Reply::Movement(v) => v,
        _ => panic!(),
    }
}
fn match_cmd(m: &Movement, x: &Candidate) -> Command {
    Command {
        request_id: key(),
        action: Action::Match {
            movement_id: m.id.clone(),
            revision: m.revision,
            target_kind: x.target_kind.clone(),
            document_id: x.document_id.clone(),
            target_revision: x.revision,
            payment_id: x.payment_id.clone(),
        },
    }
}
async fn unlink(s: &PgBank, a: &str, c: &str, m: &Movement) -> Movement {
    match run(
        s,
        a,
        c,
        Action::Unmatch {
            movement_id: m.id.clone(),
            revision: m.revision,
            reason: "Synthetic correction".into(),
        },
    )
    .await
    {
        Reply::Movement(v) => v,
        _ => panic!(),
    }
}
const HEADER: &str =
    "external_id,booked_on,amount,currency,variable_symbol,counterparty,description\n";
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn import_atomic_dedup_concurrency_account_scope_and_roles(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let csv=format!("{HEADER}A,2026-10-01,100.00,CZK,001,Vendor,First\nA,2026-10-01,100.00,CZK,001,Vendor,First\nB,2026-10-01,-100.00,CZK,,Vendor,Second\n");
    let f = || banking::FileInput {
        account_id: ac.id.clone(),
        csv: csv.clone(),
    };
    let v = s.preview(&a, &c, f()).await.unwrap();
    assert!(v.valid);
    assert_eq!(v.new_count, 2);
    assert_eq!(v.duplicate_count, 1);
    assert_eq!(v.rows[0].data.variable_symbol, "001");
    assert!(s.preview(&a, &other, f()).await.is_err());
    let cmd = || Command {
        request_id: key(),
        action: Action::Import {
            account_id: ac.id.clone(),
            csv: csv.clone(),
        },
    };
    let (x, y) = tokio::join!(s.execute(&a, &c, cmd()), s.execute(&a, &c, cmd()));
    assert!(x.is_ok() && y.is_ok());
    let all = s.movements(&a, &c, Default::default()).await.unwrap();
    assert_eq!(all.total, 2);
    assert_ne!(all.items[0].number, all.items[1].number);
    assert!(all.items.iter().all(|v| v.matches.is_empty()));
    assert_eq!(s.preview(&a, &c, f()).await.unwrap().duplicate_count, 3);
    let bad = format!("{HEADER}NEW,2026-10-01,1,CZK,,,\nA,2026-10-01,101,CZK,001,Vendor,First\n");
    let v = s
        .preview(
            &a,
            &c,
            banking::FileInput {
                account_id: ac.id.clone(),
                csv: bad.clone(),
            },
        )
        .await
        .unwrap();
    assert!(!v.valid);
    assert_eq!(v.rows[1].status, "conflict");
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Import {
                    account_id: ac.id.clone(),
                    csv: bad
                }
            }
        )
        .await
        .is_err());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        2
    );
    for row in [
        "X,2026-10-01,1.001,CZK,,,",
        "X,2099-10-01,1,CZK,,,",
        "X,2026-10-01,0,CZK,,,",
        "X,2026-10-01,1,EUR,,,",
    ] {
        let r = s
            .preview(
                &a,
                &c,
                banking::FileInput {
                    account_id: ac.id.clone(),
                    csv: format!("{HEADER}{row}"),
                },
            )
            .await;
        assert!(r.is_err() || !r.unwrap().valid);
    }
    let mut changed = ac.data.clone();
    changed.iban = "CZ5808000000001234567899".into();
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Account {
                    id: Some(ac.id.clone()),
                    revision: Some(ac.revision),
                    data: changed,
                    archived: false
                }
            }
        )
        .await
        .is_err());
    let archive = Command {
        request_id: key(),
        action: Action::Account {
            id: Some(ac.id.clone()),
            revision: Some(ac.revision),
            data: ac.data.clone(),
            archived: true,
        },
    };
    let archive_json = serde_json::to_value(&archive).unwrap();
    let one = s.execute(&a, &c, archive).await.unwrap();
    let two = s
        .execute(&a, &c, serde_json::from_value(archive_json).unwrap())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(one).unwrap(),
        serde_json::to_value(two).unwrap()
    );
    assert!(s.preview(&a, &c, f()).await.is_err());
    assert!(s.get(&a, &other, &all.items[0].id).await.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.accounts(&a, &c).await.is_ok());
    assert!(s.execute(&a, &c, cmd()).await.is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn incoming_new_and_existing_payments_reversal_and_replay(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 0, false).await;
    let payments = PgPayment(p.clone());
    let mut m = movement(&s, &a, &c, &ac, "100").await;
    // Set movement date to the invoice's current issue date through a new legitimate movement.
    let data = MovementData {
        booked_on: d.data.issued_on.unwrap(),
        external_id: "".into(),
        ..m.data.clone()
    };
    m = match run(
        &s,
        &a,
        &c,
        Action::Manual {
            account_id: ac.id.clone(),
            data,
        },
    )
    .await
    {
        Reply::Movement(v) => v,
        _ => panic!(),
    };
    let candidates = s.candidates(&a, &c, &m.id, "").await.unwrap();
    let x = candidates
        .iter()
        .find(|x| x.document_id == d.id && x.payment_id.is_none())
        .unwrap();
    let cmd = match_cmd(&m, x);
    let json = serde_json::to_value(&cmd).unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, cmd),
        s.execute(&a, &c, serde_json::from_value(json.clone()).unwrap())
    );
    let one = one.unwrap();
    assert_eq!(
        serde_json::to_value(&one).unwrap(),
        serde_json::to_value(two.unwrap()).unwrap()
    );
    let m = match one {
        Reply::Movement(v) => v,
        _ => panic!(),
    };
    let paid = payments.get(&a, &c, &d.id).await.unwrap();
    assert_eq!(paid.summary.paid, "100.00");
    assert_eq!(paid.items.len(), 1);
    assert!(s
        .execute(&a, &other, serde_json::from_value(json).unwrap())
        .await
        .is_err());
    let e = payment::reverse(
        &payments,
        &a,
        &c,
        &d.id,
        &paid.items[0].id,
        payment::Reverse {
            request_id: key(),
            revision: paid.summary.revision,
            reason: "Forbidden direct reverse".into(),
            replacement: None,
        },
    )
    .await;
    assert!(
        matches!(e,Err(company::CompanyError::Invalid(ref v)) if v.contains(&"bank_link".into()))
    );
    let m = unlink(&s, &a, &c, &m).await;
    assert!(m.matches[0].detached_at.is_some());
    assert_eq!(
        payments.get(&a, &c, &d.id).await.unwrap().summary.paid,
        "0.00"
    );
    let old = payments.get(&a, &c, &d.id).await.unwrap();
    let paid = payment::register(
        &payments,
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: old.summary.revision,
            payment: PaymentInput {
                amount: "100.00".into(),
                paid_on: d.data.issued_on.unwrap(),
                method: "bank_transfer".into(),
                note: "Previously recorded".into(),
            },
        },
    )
    .await
    .unwrap();
    let candidates = s.candidates(&a, &c, &m.id, "").await.unwrap();
    let existing = candidates.iter().find(|x| x.payment_id.is_some()).unwrap();
    let m = match s.execute(&a, &c, match_cmd(&m, existing)).await.unwrap() {
        Reply::Movement(v) => v,
        _ => panic!(),
    };
    assert!(
        !m.matches
            .iter()
            .find(|x| x.detached_at.is_none())
            .unwrap()
            .created_payment
    );
    assert_eq!(
        payments.get(&a, &c, &d.id).await.unwrap().items.len(),
        paid.items.len()
    );
    unlink(&s, &a, &c, &m).await;
    assert_eq!(
        payments.get(&a, &c, &d.id).await.unwrap().summary.paid,
        "100.00"
    );
    let too_big = movement(&s, &a, &c, &ac, "999999").await;
    assert!(s
        .candidates(&a, &c, &too_big.id, "")
        .await
        .unwrap()
        .is_empty());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn outgoing_received_and_card_receipt_preserve_cash_ledger(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let received = PgReceived(p.clone());
    let invoice = received::create(
        &received,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let m = movement(&s, &a, &c, &ac, "-21").await;
    let candidates = s.candidates(&a, &c, &m.id, "").await.unwrap();
    let x = candidates
        .iter()
        .find(|x| x.document_id == invoice.id)
        .unwrap();
    let m = match s.execute(&a, &c, match_cmd(&m, x)).await.unwrap() {
        Reply::Movement(v) => v,
        _ => panic!(),
    };
    assert_eq!(
        received.get(&a, &c, &invoice.id).await.unwrap().paid,
        "21.00"
    );
    unlink(&s, &a, &c, &m).await;
    assert_eq!(
        received.get(&a, &c, &invoice.id).await.unwrap().paid,
        "0.00"
    );
    let mut d = data();
    d.kind = faktura_domain::received::Kind::Receipt;
    d.receipt_method = Some("card".into());
    d.received_on = d.issued_on;
    d.due_on = d.issued_on;
    d.variable_symbol.clear();
    d.bank_account.clear();
    let receipt = received::create(
        &received,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: d.clone(),
        },
    )
    .await
    .unwrap();
    let m = movement(&s, &a, &c, &ac, "-121").await;
    let candidates = s.candidates(&a, &c, &m.id, "").await.unwrap();
    let x = candidates
        .iter()
        .find(|x| x.document_id == receipt.id)
        .unwrap();
    assert!(x.payment_id.is_some());
    let m = match s.execute(&a, &c, match_cmd(&m, x)).await.unwrap() {
        Reply::Movement(v) => v,
        _ => panic!(),
    };
    assert_eq!(
        received
            .get(&a, &c, &receipt.id)
            .await
            .unwrap()
            .payments
            .len(),
        1
    );
    assert!(received::command(
        &received,
        &a,
        &c,
        &receipt.id,
        received::Command {
            request_id: key(),
            revision: receipt.revision,
            action: received::Action::Update {
                data: Box::new(d.clone())
            }
        }
    )
    .await
    .is_err());
    unlink(&s, &a, &c, &m).await;
    assert_eq!(
        received.get(&a, &c, &receipt.id).await.unwrap().paid,
        "121.00"
    );
    d.receipt_method = Some("cash".into());
    let cash = received::create(
        &received,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: d,
        },
    )
    .await
    .unwrap();
    assert!(!s
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .iter()
        .any(|v| v.document_id == cash.id));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_payment")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_parity_account_update_and_origin(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let Reply::Account(ac) = run(
        &s,
        &a,
        &c,
        Action::Account {
            id: None,
            revision: None,
            archived: false,
            data: AccountData {
                name: "Foreign".into(),
                iban: "GB82WEST12345698765432".into(),
                currency: "CZK".into(),
            },
        },
    )
    .await
    else {
        panic!()
    };
    let m = movement(&s, &a, &c, &ac, "12").await;
    let app =
        faktura_server::banking::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/bank/movements/{}", m.id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
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
    use base64::Engine;
    let native = serde_json::json!({"account_id":ac.id,"format":"camt053","filename":"synthetic.xml","content_base64":base64::engine::general_purpose::STANDARD.encode(include_str!("../../../tests/fixtures/bank/camt053.xml").replace("CZ6508000000192000145399", &ac.data.iban))});
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/bank/statement-preview"))
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(native.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let native_http: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    let mut native_args = native;
    native_args["company_id"] = c.clone().into();
    let native_mcp = client
        .call_tool(
            CallToolRequestParams::new("bank_statement_preview")
                .with_arguments(native_args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(native_mcp.is_error, Some(true));
    let json = serde_json::to_value(native_mcp).unwrap();
    let native_result: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(native_result, native_http);
    let suggested = client
        .call_tool(
            CallToolRequestParams::new("bank_suggestions").with_arguments(
                serde_json::json!({"company_id":c,"id":m.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(suggested.is_error, Some(true));

    let result = client
        .call_tool(
            CallToolRequestParams::new("bank_get").with_arguments(
                serde_json::json!({"company_id":c,"id":m.id})
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
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/bank/balance?account_id={}",
                    ac.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let http_balance: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_balance").with_arguments(
                serde_json::json!({"company_id":c,"account_id":ac.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let v = serde_json::to_value(r).unwrap();
    let mcp_balance: serde_json::Value =
        serde_json::from_str(v["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http_balance, mcp_balance);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/bank/connection?account_id={}",
                    ac.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let http_connection: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_connection_status").with_arguments(
                serde_json::json!({"company_id":c,"account_id":ac.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let v = serde_json::to_value(r).unwrap();
    let mcp_connection: serde_json::Value =
        serde_json::from_str(v["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http_connection, mcp_connection);

    let cmd = Command {
        request_id: key(),
        action: Action::Account {
            id: Some(ac.id.clone()),
            revision: Some(ac.revision),
            data: AccountData {
                name: "Renamed through MCP".into(),
                ..ac.data.clone()
            },
            archived: false,
        },
    };
    let mut args = serde_json::to_value(&cmd).unwrap();
    args["company_id"] = c.clone().into();
    let result = client
        .call_tool(
            CallToolRequestParams::new("bank_execute")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    assert_eq!(
        s.accounts(&a, &c).await.unwrap()[0].data.name,
        "Renamed through MCP"
    );
    assert_eq!(s.accounts(&a, &c).await.unwrap().len(), 1);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/bank/commands"))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .header("origin", "https://foreign.example.test")
                .body(Body::from(serde_json::to_vec(&cmd).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/bank/commands"))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .header("origin", "http://localhost:5173")
                .header("X-Faktura-Request", "1")
                .body(Body::from(serde_json::to_vec(&cmd).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_026_preserves_existing_payment_and_receipt(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610130027)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let d = issued_case(&p, &a, &c, 0, false).await;
    let s = PgPayment(p.clone());
    let paid = payment::register(
        &s,
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: 0,
            payment: PaymentInput {
                amount: "10.00".into(),
                paid_on: d.data.issued_on.unwrap(),
                method: "bank_transfer".into(),
                note: "Before bank migration".into(),
            },
        },
    )
    .await
    .unwrap();
    let mut input = data();
    input.kind = faktura_domain::received::Kind::Receipt;
    input.receipt_method = Some("card".into());
    input.received_on = input.issued_on;
    input.due_on = input.issued_on;
    input.variable_symbol.clear();
    input.bank_account.clear();
    let r = received::create(
        &PgReceived(p.clone()),
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: input,
        },
    )
    .await
    .unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    assert_eq!(
        serde_json::to_value(s.get(&a, &c, &d.id).await.unwrap()).unwrap(),
        serde_json::to_value(paid).unwrap()
    );
    assert_eq!(
        serde_json::to_value(PgReceived(p.clone()).get(&a, &c, &r.id).await.unwrap()).unwrap(),
        serde_json::to_value(r).unwrap()
    );
    assert!(PgBank(p).accounts(&a, &c).await.unwrap().is_empty());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn one_existing_payment_cannot_be_matched_twice(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 0, false).await;
    let paid = payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: 0,
            payment: PaymentInput {
                amount: "25.00".into(),
                paid_on: d.data.issued_on.unwrap(),
                method: "bank_transfer".into(),
                note: "Synthetic existing payment".into(),
            },
        },
    )
    .await
    .unwrap();
    let m1 = movement(&s, &a, &c, &ac, "25").await;
    let m2 = movement(&s, &a, &c, &ac, "25").await;
    let choices = s.candidates(&a, &c, &m1.id, "").await.unwrap();
    let candidate = choices
        .iter()
        .find(|v| v.payment_id.as_deref() == Some(&paid.items[0].id))
        .unwrap();
    let wrong_company = s.execute(&a, &other, match_cmd(&m1, candidate)).await;
    assert!(wrong_company.is_err());
    let negative = movement(&s, &a, &c, &ac, "-25").await;
    assert!(s
        .execute(&a, &c, match_cmd(&negative, candidate))
        .await
        .is_err());
    let (x, y) = tokio::join!(
        s.execute(&a, &c, match_cmd(&m1, candidate)),
        s.execute(&a, &c, match_cmd(&m2, candidate))
    );
    assert_eq!(u8::from(x.is_ok()) + u8::from(y.is_ok()), 1);
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let matched = s
        .movements(
            &a,
            &c,
            banking::Search {
                status: "matched".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(matched.total, 1);
    let foreign_doc = issued_case(&p, &a, &other, 0, false).await;
    let mut foreign = match_cmd(&negative, candidate);
    if let Action::Match { document_id, .. } = &mut foreign.action {
        *document_id = foreign_doc.id;
    }
    assert!(s.execute(&a, &c, foreign).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_statement_atomic_replay_and_format_isolation(p: PgPool) {
    use base64::Engine;
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let file = || banking::StatementInput {
        account_id: ac.id.clone(),
        format: "camt053".into(),
        filename: "synthetic.xml".into(),
        content_base64: base64::engine::general_purpose::STANDARD
            .encode(include_str!("../../../tests/fixtures/bank/camt053.xml")),
    };
    let report = s.statement_preview(&a, &c, file()).await.unwrap();
    assert_eq!(report.new_count, 2);
    assert!(report.valid);
    assert!(s.statement_preview(&a, &other, file()).await.is_err());
    let mut invalid = file();
    invalid.content_base64 = base64::engine::general_purpose::STANDARD.encode(
        include_str!("../../../tests/fixtures/bank/camt053.xml").replace("25.00", "25.001"),
    );
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Statement { file: invalid }
            }
        )
        .await
        .is_err());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        0
    );
    let cmd = Command {
        request_id: key(),
        action: Action::Statement { file: file() },
    };
    let wire = serde_json::to_value(&cmd).unwrap();
    let decoded: Command = serde_json::from_value(wire.clone()).unwrap();
    s.execute(&a, &c, decoded).await.unwrap();
    s.execute(&a, &c, serde_json::from_value(wire).unwrap())
        .await
        .unwrap();
    let (x, y) = tokio::join!(
        s.execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Statement { file: file() }
            }
        ),
        s.execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Statement { file: file() }
            }
        )
    );
    assert!(x.is_ok() && y.is_ok());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        2
    );
    assert_eq!(
        s.statement_preview(&a, &c, file())
            .await
            .unwrap()
            .duplicate_count,
        2
    );
    assert!(s
        .preview(
            &a,
            &c,
            banking::FileInput {
                account_id: ac.id.clone(),
                csv: format!("{HEADER}\nx,2026-10-01,100,CZK,,,\n")
            }
        )
        .await
        .is_err());
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_payment")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(total, 0);
    let mut conflict = file();
    conflict.content_base64 = base64::engine::general_purpose::STANDARD.encode(
        include_str!("../../../tests/fixtures/bank/camt053.xml").replace("100.01", "101.01"),
    );
    assert!(!s.statement_preview(&a, &c, conflict).await.unwrap().valid);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn suggestions_are_scoped_ambiguous_and_do_not_create_payments(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = match run(
        &s,
        &a,
        &c,
        Action::Manual {
            account_id: ac.id.clone(),
            data: MovementData {
                external_id: "".into(),
                booked_on: d.data.issued_on.unwrap(),
                amount: "100".into(),
                currency: "CZK".into(),
                variable_symbol: d.data.payment.variable_symbol.clone(),
                counterparty: "Test".into(),
                description: "".into(),
            },
        },
    )
    .await
    {
        Reply::Movement(m) => m,
        _ => panic!(),
    };
    let choices = s.suggestions(&a, &c, &m.id).await.unwrap();
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].document_id, d.id);
    assert!(s.suggestions(&a, &other, &m.id).await.is_err());
    let duplicate = draft::create(
        &PgDraft(p.clone()),
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data: d.data.clone(),
        },
    )
    .await
    .unwrap();
    draft::issue(
        &PgDraft(p.clone()),
        &a,
        &c,
        &duplicate.id,
        draft::Issue {
            request_id: key(),
            revision: duplicate.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(s.suggestions(&a, &c, &m.id).await.unwrap().len(), 2);
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .items
            .len(),
        0
    );
    let paid = payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: 0,
            payment: PaymentInput {
                paid_on: d.data.issued_on.unwrap(),
                amount: "100".into(),
                method: "bank_transfer".into(),
                note: "".into(),
            },
        },
    )
    .await
    .unwrap();
    let choices = s.suggestions(&a, &c, &m.id).await.unwrap();
    assert_eq!(choices.len(), 1);
    assert_eq!(
        choices[0].payment_id.as_deref(),
        Some(paid.items[0].id.as_str())
    );
    s.execute(&a, &c, match_cmd(&m, &choices[0])).await.unwrap();
    assert!(s.suggestions(&a, &c, &m.id).await.unwrap().is_empty());
}

#[path = "bank/balance.rs"]
mod balance_tests;
#[path = "bank/batch.rs"]
mod batch_tests;

#[path = "bank/connection.rs"]
mod connection;

#[path = "bank/automation.rs"]
mod automation;

#[path = "bank/split.rs"]
mod split_tests;

#[path = "bank/treasury.rs"]
mod treasury_tests;

#[path = "bank/foreign.rs"]
mod foreign_tests;

#[path = "bank/currencies.rs"]
mod currency_tests;

#[path = "bank/native_matching.rs"]
mod native_matching;

#[path = "bank/native_automation.rs"]
mod native_automation;

#[path = "bank/native_treasury.rs"]
mod native_treasury;

#[path = "bank/cross_currency.rs"]
mod cross_currency;

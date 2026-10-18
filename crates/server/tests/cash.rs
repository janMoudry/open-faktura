use faktura_application::{
    auth,
    cash::{self, Account, Action, Candidate, CashStore, Command, Movement, Reply},
    company, draft,
    payment::{self, PaymentStore},
    received::{self, ReceivedStore},
};
use faktura_domain::{
    cash::{AccountData, MovementData},
    company::CompanySettings,
    invoice::{DraftData, PartySnapshot, TaxMode},
    payment::PaymentInput,
    received::{Data, Line},
};
use faktura_infrastructure::{
    auth::PgAuth, cash::PgCash, company::PgCompany, draft::PgDraft, payment::PgPayment,
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
async fn run(s: &PgCash, a: &str, c: &str, action: Action) -> Reply {
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
async fn account(s: &PgCash, a: &str, c: &str) -> Account {
    match run(
        s,
        a,
        c,
        Action::Account {
            id: None,
            revision: None,
            data: AccountData {
                name: "Operating".into(),
                opening_on: "2026-01-01".parse().unwrap(),
                opening_balance: "1000.00".into(),
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
async fn movement(s: &PgCash, a: &str, c: &str, account: &Account, amount: &str) -> Movement {
    match run(
        s,
        a,
        c,
        Action::Manual {
            account_id: account.id.clone(),
            data: MovementData {
                person: "Cashier".into(),
                booked_on: "2026-10-01".parse().unwrap(),
                amount: amount.into(),
                currency: "CZK".into(),
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
async fn unlink(s: &PgCash, a: &str, c: &str, m: &Movement) -> Movement {
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
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn incoming_new_and_existing_payments_reversal_and_replay(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 0, false).await;
    let payments = PgPayment(p.clone());
    let mut m = movement(&s, &a, &c, &ac, "100").await;
    // Set movement date to the invoice's current issue date through a new legitimate movement.
    let data = MovementData {
        booked_on: d.data.issued_on.unwrap(),
        person: "Cashier".into(),
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
        matches!(e,Err(company::CompanyError::Invalid(ref v)) if v.contains(&"cash_link".into()))
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
                method: "cash".into(),
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
    let s = PgCash(p.clone());
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
    d.receipt_method = Some("cash".into());
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
    d.receipt_method = Some("card".into());
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
async fn balance_numbering_cancel_scope_roles_and_replay(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "100.01").await;
    assert_eq!(m.number, "PP2026000001");
    let e = movement(&s, &a, &c, &ac, "-40.02").await;
    assert_eq!(e.number, "VP2026000001");
    let query = || cash::Search {
        account_id: ac.id.clone(),
        ..Default::default()
    };
    let b = s.balance(&a, &c, query()).await.unwrap();
    assert_eq!(
        (
            b.opening.as_str(),
            b.income.as_str(),
            b.expense.as_str(),
            b.closing.as_str()
        ),
        ("1000.00", "100.01", "40.02", "1059.99")
    );
    assert!(s.balance(&a, &other, query()).await.is_err());
    assert!(s.get(&a, &other, &m.id).await.is_err());
    let cmd = Command {
        request_id: key(),
        action: Action::Cancel {
            movement_id: e.id.clone(),
            revision: 1,
            reason: "Mistake".into(),
        },
    };
    let json = serde_json::to_value(&cmd).unwrap();
    let first = s.execute(&a, &c, cmd).await.unwrap();
    let second = s
        .execute(&a, &c, serde_json::from_value(json).unwrap())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(second).unwrap()
    );
    assert_eq!(s.balance(&a, &c, query()).await.unwrap().closing, "1100.01");
    let mut changed = ac.data.clone();
    changed.opening_balance = "5".into();
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Account {
                    id: Some(ac.id.clone()),
                    revision: Some(1),
                    data: changed,
                    archived: false
                }
            }
        )
        .await
        .is_err());
    let b = s
        .balance(
            &a,
            &c,
            cash::Search {
                from: Some("2026-10-02".parse().unwrap()),
                ..query()
            },
        )
        .await
        .unwrap();
    assert_eq!(b.opening, "1100.01");
    assert_eq!(b.count, 0);
    for amount in ["0", "0.001", "1000000000000", "NaN"] {
        let mut d = m.data.clone();
        d.amount = amount.into();
        assert!(s
            .execute(
                &a,
                &c,
                Command {
                    request_id: key(),
                    action: Action::Manual {
                        account_id: ac.id.clone(),
                        data: d
                    }
                }
            )
            .await
            .is_err());
    }
    let mut d = m.data.clone();
    d.booked_on = "2025-12-31".parse().unwrap();
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Manual {
                    account_id: ac.id.clone(),
                    data: d
                }
            }
        )
        .await
        .is_err());
    // Summary must include entries beyond the 25-row register page.
    for _ in 0..26 {
        movement(&s, &a, &c, &ac, "1.00").await;
    }
    assert_eq!(s.movements(&a, &c, query()).await.unwrap().items.len(), 25);
    assert_eq!(s.balance(&a, &c, query()).await.unwrap().closing, "1126.01");
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.balance(&a, &c, query()).await.is_ok());
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Cancel {
                    movement_id: m.id.clone(),
                    revision: m.revision,
                    reason: "Denied".into()
                }
            }
        )
        .await
        .is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_numbering_and_failed_command_rollback(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    let (m, n) = tokio::join!(
        movement(&s, &a, &c, &ac, "10"),
        movement(&s, &a, &c, &ac, "20")
    );
    assert_ne!(m.number, n.number);
    let mut input = m.data.clone();
    input.amount = "30".into();
    let cmd = Command {
        request_id: key(),
        action: Action::Manual {
            account_id: ac.id.clone(),
            data: input,
        },
    };
    let json = serde_json::to_value(&cmd).unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, cmd),
        s.execute(&a, &c, serde_json::from_value(json.clone()).unwrap())
    );
    assert_eq!(
        serde_json::to_value(one.unwrap()).unwrap(),
        serde_json::to_value(two.unwrap()).unwrap()
    );
    let mut changed = json;
    changed["data"]["amount"] = "31".into();
    assert!(s
        .execute(&a, &c, serde_json::from_value(changed).unwrap())
        .await
        .is_err());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        3
    );
    // A rejected write must not consume a sequence value or leave an audit command.
    sqlx::raw_sql("CREATE FUNCTION fail_cash() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic failure'; END; $$; CREATE TRIGGER fail_cash BEFORE INSERT ON cash_movement FOR EACH ROW EXECUTE FUNCTION fail_cash();").execute(&p).await.unwrap();
    let cmd = Command {
        request_id: key(),
        action: Action::Manual {
            account_id: ac.id.clone(),
            data: m.data.clone(),
        },
    };
    let retry = serde_json::to_value(&cmd).unwrap();
    assert!(s.execute(&a, &c, cmd).await.is_err());
    sqlx::query("DROP TRIGGER fail_cash ON cash_movement")
        .execute(&p)
        .await
        .unwrap();
    let r = s
        .execute(&a, &c, serde_json::from_value(retry).unwrap())
        .await
        .unwrap();
    let Reply::Movement(r) = r else { panic!() };
    assert_eq!(r.number, "PP2026000004");
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
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "12").await;
    let app = faktura_server::cash::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/cash/movements/{}", m.id))
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
    let result = client
        .call_tool(
            CallToolRequestParams::new("cash_get").with_arguments(
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
    let terminal_result=client.call_tool(CallToolRequestParams::new("cash_execute").with_arguments(serde_json::json!({"company_id":c,"request_id":key(),"action":"terminal","id":null,"revision":null,"code":"POS1","name":"Synthetic POS","archived":false}).as_object().unwrap().clone())).await.unwrap();
    assert_ne!(terminal_result.is_error, Some(true));
    let terminal_list = client
        .call_tool(
            CallToolRequestParams::new("cash_terminals").with_arguments(
                serde_json::json!({"company_id":c})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(terminal_list.is_error, Some(true));
    let terminal_json = serde_json::to_value(terminal_list).unwrap();
    let terminal_mcp: serde_json::Value =
        serde_json::from_str(terminal_json["content"][0]["text"].as_str().unwrap()).unwrap();
    let terminal_http = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/cash/terminals"))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(terminal_http.status(), 200);
    let terminal_http: serde_json::Value =
        serde_json::from_slice(&to_bytes(terminal_http.into_body(), 100000).await.unwrap())
            .unwrap();
    assert_eq!(terminal_mcp, terminal_http);
    assert_eq!(terminal_http[0]["code"], "POS1");

    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{tax_mode}',to_jsonb('vat_payer'::text)) WHERE id=$1::text::bigint").bind(&c).execute(&p).await.unwrap();
    let preview = serde_json::json!({"amount":"100","tax":{"price_mode":"net","vat_percent":"21","tax_on":"2026-10-01"}});
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/cash/tax-preview"))
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(preview.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let http_preview: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
    let mut arguments = preview.as_object().unwrap().clone();
    arguments.insert("company_id".into(), c.clone().into());
    let result = client
        .call_tool(CallToolRequestParams::new("cash_tax_preview").with_arguments(arguments))
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let result = serde_json::to_value(result).unwrap();
    let mcp_preview: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http_preview, mcp_preview);
    assert_eq!(http_preview["amounts"]["gross"], "121.00");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/cash/movements/{}/pdf?language=en",
                    m.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "application/pdf");
    let bytes = to_bytes(response.into_body(), 10_000_000).await.unwrap();
    let r = client
        .call_tool(
            CallToolRequestParams::new("cash_pdf").with_arguments(
                serde_json::json!({"company_id":c,"id":m.id,"language":"en"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let json = serde_json::to_value(r).unwrap();
    let pdf: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    use base64::Engine;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(pdf["content_base64"].as_str().unwrap())
            .unwrap(),
        bytes
    );
    let r = client
        .call_tool(
            CallToolRequestParams::new("cash_settings").with_arguments(
                serde_json::json!({"company_id":c})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
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
            CallToolRequestParams::new("cash_execute")
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
                .uri(format!("/api/companies/{c}/cash/commands"))
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
                .uri(format!("/api/companies/{c}/cash/commands"))
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
async fn upgrade_028_preserves_existing_payment_and_receipt(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140029)
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
    assert!(PgCash(p).accounts(&a, &c).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn one_existing_payment_cannot_be_matched_twice(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
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
                paid_on: "2026-10-01".parse().unwrap(),
                method: "cash".into(),
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
            cash::Search {
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

#[path = "cash/delivery.rs"]
mod delivery_tests;
#[path = "cash/tax.rs"]
mod tax_tests;

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn terminals_scope_revision_replay_archive_and_roles(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgCash(p.clone());
    let cmd = Command {
        request_id: key(),
        action: Action::Terminal {
            id: None,
            revision: None,
            code: "front_1".into(),
            name: "Front desk".into(),
            archived: false,
        },
    };
    let input = serde_json::to_value(&cmd).unwrap();
    let Reply::Terminal(v) = s.execute(&a, &c, cmd).await.unwrap() else {
        panic!()
    };
    assert_eq!(v.code, "FRONT_1");
    let Reply::Terminal(replay) = s
        .execute(&a, &c, serde_json::from_value(input).unwrap())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(v.id, replay.id);
    assert!(s.terminals(&a, &other).await.unwrap().is_empty());
    let change = |company_id: Option<String>, revision: Option<i64>, code: &str| Command {
        request_id: key(),
        action: Action::Terminal {
            id: company_id,
            revision,
            code: code.into(),
            name: "Updated".into(),
            archived: true,
        },
    };
    assert!(s
        .execute(
            &a,
            &other,
            change(Some(v.id.clone()), Some(v.revision), "FRONT_1")
        )
        .await
        .is_err());
    assert!(s
        .execute(&a, &c, change(None, None, "FRONT_1"))
        .await
        .is_err());
    let Reply::Terminal(updated) = s
        .execute(
            &a,
            &c,
            change(Some(v.id.clone()), Some(v.revision), "FRONT_1"),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(updated.archived);
    assert_eq!(updated.revision, 2);
    assert!(s
        .execute(
            &a,
            &c,
            change(Some(v.id.clone()), Some(v.revision), "FRONT_1")
        )
        .await
        .is_err());
    assert!(s
        .execute(&a, &c, change(None, None, "bad code"))
        .await
        .is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert_eq!(s.terminals(&a, &c).await.unwrap().len(), 1);
    assert!(s
        .execute(&a, &c, change(None, None, "SECOND"))
        .await
        .is_err());
}

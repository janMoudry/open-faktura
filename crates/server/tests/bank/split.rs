use super::batch_tests::for_invoice;
use super::*;
use banking::Allocation;
fn alloc(d: &draft::Draft, value: &str) -> Allocation {
    Allocation {
        target_kind: "invoice".into(),
        document_id: d.id.clone(),
        target_revision: 0,
        payment_id: None,
        amount: value.into(),
    }
}
async fn allocation(p: &PgPool, a: &str, c: &str, d: &draft::Draft, value: &str) -> Allocation {
    let mut v = alloc(d, value);
    v.target_revision = PgPayment(p.clone())
        .get(a, c, &d.id)
        .await
        .unwrap()
        .summary
        .revision;
    v
}
fn split(m: &Movement, allocations: Vec<Allocation>) -> Command {
    Command {
        request_id: key(),
        action: Action::Split {
            movement_id: m.id.clone(),
            revision: m.revision,
            allocations,
        },
    }
}
fn result(r: Reply) -> Movement {
    if let Reply::Movement(m) = r {
        m
    } else {
        panic!()
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn split_incoming_decimal_remainder_replay_and_unmatch(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let m = for_invoice(&s, &a, &c, &ac, &d1, "650.01").await;
    assert!(s.candidates(&a, &c, &m.id, "").await.unwrap().is_empty());
    assert_eq!(
        s.split_candidates(&a, &c, &m.id, "").await.unwrap().len(),
        2
    );
    let cmd = split(
        &m,
        vec![
            allocation(&p, &a, &c, &d1, "300").await,
            allocation(&p, &a, &c, &d2, "250.02").await,
        ],
    );
    let saved = serde_json::to_value(&cmd).unwrap();
    let one = result(s.execute(&a, &c, cmd).await.unwrap());
    assert_eq!(one.allocated, "550.02");
    assert_eq!(one.remaining, "99.99");
    assert_eq!(one.matches.len(), 2);
    let again = result(
        s.execute(&a, &c, serde_json::from_value(saved.clone()).unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(again.revision, one.revision);
    let mut changed = saved;
    changed["allocations"][0]["amount"] = "299".into();
    assert!(s
        .execute(&a, &c, serde_json::from_value(changed).unwrap())
        .await
        .is_err());
    let partial = s
        .movements(
            &a,
            &c,
            banking::Search {
                status: "partial".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(partial.total, 1);
    assert_eq!(
        s.movements(
            &a,
            &c,
            banking::Search {
                status: "matched".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    assert!(s.suggestions(&a, &c, &m.id).await.unwrap().is_empty());
    let two = result(
        s.execute(
            &a,
            &c,
            split(&one, vec![allocation(&p, &a, &c, &d2, "49.98").await]),
        )
        .await
        .unwrap(),
    );
    assert_eq!(two.remaining, "50.01");
    let unlinked = unlink(&s, &a, &c, &two).await;
    assert_eq!(unlinked.allocated, "0.00");
    assert_eq!(unlinked.remaining, "650.01");
    assert!(unlinked.matches.iter().all(|v| v.detached_at.is_some()));
    for d in [&d1, &d2] {
        assert_eq!(
            PgPayment(p.clone())
                .get(&a, &c, &d.id)
                .await
                .unwrap()
                .summary
                .paid,
            "0.00"
        );
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn split_existing_payment_and_concurrent_confirmation(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let payments = PgPayment(p.clone());
    let rev = payments.get(&a, &c, &d1.id).await.unwrap().summary.revision;
    let paid = payment::register(
        &payments,
        &a,
        &c,
        &d1.id,
        payment::Register {
            request_id: key(),
            revision: rev,
            payment: PaymentInput {
                amount: "100.01".into(),
                paid_on: d1.data.issued_on.unwrap(),
                method: "bank_transfer".into(),
                note: "Existing synthetic".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = for_invoice(&s, &a, &c, &ac, &d1, "150.02").await;
    let existing = s
        .split_candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.payment_id.as_deref() == Some(paid.items[0].id.as_str()))
        .unwrap();
    assert_eq!(existing.available, "100.01");
    let first = Allocation {
        target_kind: existing.target_kind,
        document_id: existing.document_id,
        target_revision: existing.revision,
        payment_id: existing.payment_id,
        amount: existing.available,
    };
    let command = split(&m, vec![first, allocation(&p, &a, &c, &d2, "50.01").await]);
    let value = serde_json::to_value(&command).unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, command),
        s.execute(&a, &c, serde_json::from_value(value).unwrap())
    );
    let one = result(one.unwrap());
    two.unwrap();
    assert_eq!(one.remaining, "0.00");
    assert_eq!(payments.get(&a, &c, &d1.id).await.unwrap().items.len(), 1);
    let detached = unlink(&s, &a, &c, &one).await;
    assert_eq!(detached.matches.len(), 2);
    assert_eq!(
        payments.get(&a, &c, &d1.id).await.unwrap().summary.paid,
        "100.01"
    );
    assert_eq!(
        payments.get(&a, &c, &d2.id).await.unwrap().summary.paid,
        "0.00"
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn split_outgoing_existing_card_and_new_received_payment(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let received = PgReceived(p.clone());
    let d = received
        .create(
            &a,
            &c,
            received::Create {
                request_id: key(),
                data: data(),
            },
        )
        .await
        .unwrap();
    let mut receipt = data();
    receipt.kind = faktura_domain::received::Kind::Receipt;
    receipt.receipt_method = Some("card".into());
    receipt.received_on = receipt.issued_on;
    receipt.due_on = receipt.issued_on;
    receipt.variable_symbol.clear();
    receipt.bank_account.clear();
    let card = received
        .create(
            &a,
            &c,
            received::Create {
                request_id: key(),
                data: receipt,
            },
        )
        .await
        .unwrap();
    let m = movement(&s, &a, &c, &ac, "-200.00").await;
    let choices = s.split_candidates(&a, &c, &m.id, "").await.unwrap();
    let old = choices
        .iter()
        .find(|v| v.document_id == card.id && v.payment_id.is_some())
        .unwrap();
    let new = choices
        .iter()
        .find(|v| v.document_id == d.id && v.payment_id.is_none())
        .unwrap();
    let parts = [(old, "121.00"), (new, "79.00")]
        .into_iter()
        .map(|(v, value)| Allocation {
            target_kind: v.target_kind.clone(),
            document_id: v.document_id.clone(),
            target_revision: v.revision,
            payment_id: v.payment_id.clone(),
            amount: value.into(),
        })
        .collect();
    let matched = result(s.execute(&a, &c, split(&m, parts)).await.unwrap());
    assert_eq!(matched.remaining, "0.00");
    unlink(&s, &a, &c, &matched).await;
    assert_eq!(
        received.get(&a, &c, &card.id).await.unwrap().remaining,
        "0.00"
    );
    assert_eq!(
        received.get(&a, &c, &d.id).await.unwrap().remaining,
        "121.00"
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn split_invalid_scope_roles_limits_and_atomic_failure(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let foreign = issued_case(&p, &a, &other, 20, false).await;
    let m = for_invoice(&s, &a, &c, &ac, &d1, "500").await;
    for value in ["0", "-1", "0.001", "501", "9999999999999", "NaN"] {
        assert!(s
            .execute(
                &a,
                &c,
                split(&m, vec![allocation(&p, &a, &c, &d1, value).await])
            )
            .await
            .is_err());
    }
    assert!(s
        .execute(
            &a,
            &c,
            split(&m, vec![allocation(&p, &a, &other, &foreign, "10").await])
        )
        .await
        .is_err());
    assert!(s.split_candidates(&a, &other, &m.id, "").await.is_err());
    assert!(s
        .execute(
            &a,
            &c,
            split(
                &m,
                vec![
                    allocation(&p, &a, &c, &d1, "10").await,
                    allocation(&p, &a, &c, &d1, "10").await
                ]
            )
        )
        .await
        .is_err());
    let second = d2.id.parse::<i64>().unwrap();
    sqlx::raw_sql(&format!("CREATE FUNCTION split_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.invoice_id={second} THEN RAISE EXCEPTION 'synthetic'; END IF; RETURN NEW; END $$; CREATE TRIGGER split_fail BEFORE INSERT ON invoice_payment FOR EACH ROW EXECUTE FUNCTION split_fail();")).execute(&p).await.unwrap();
    assert!(s
        .execute(
            &a,
            &c,
            split(
                &m,
                vec![
                    allocation(&p, &a, &c, &d1, "100").await,
                    allocation(&p, &a, &c, &d2, "100").await
                ]
            )
        )
        .await
        .is_err());
    assert!(s.get(&a, &c, &m.id).await.unwrap().matches.is_empty());
    assert!(PgPayment(p.clone())
        .get(&a, &c, &d1.id)
        .await
        .unwrap()
        .items
        .is_empty());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s
        .execute(
            &a,
            &c,
            split(&m, vec![allocation(&p, &a, &c, &d1, "10").await])
        )
        .await
        .is_err());
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade035_preserves_allocation_and_historical_replay(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140036)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = for_invoice(&s, &a, &c, &ac, &d, "123.45").await;
    sqlx::query("INSERT INTO bank_match(company_id,movement_id,invoice_id,payment_id,created_payment,actor_id) VALUES($1::text::bigint,$2::text::bigint,$3::text::bigint,'historical',false,$4::text::bigint)").bind(&c).bind(&m.id).bind(&d.id).bind(&a).execute(&p).await.unwrap();
    // Downgrade the manual command snapshot to the exact pre-036 JSON shape.
    sqlx::query("UPDATE bank_command SET result=jsonb_set(result,'{value}',(result->'value')-'allocated'-'remaining') WHERE result->>'kind'='movement'").execute(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    // Deployment requires a new API process: discard pre-migration SELECT * plans.
    let options = (*p.connect_options()).clone();
    p.close().await;
    let p = PgPool::connect_with(options).await.unwrap();
    let s = PgBank(p.clone());
    let upgraded = s.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(upgraded.remaining, "0.00");
    assert_eq!(upgraded.matches[0].amount, "123.45");
    let snapshot: serde_json::Value = sqlx::query_scalar(
        "SELECT result FROM bank_command WHERE result->>'kind'='movement' LIMIT 1",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    let historical: Reply = serde_json::from_value(snapshot).unwrap();
    let historical = result(historical);
    assert_eq!(historical.remaining, "123.45");
    assert!(historical.matches.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn split_rest_mcp_preview_execute_replay(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = for_invoice(&s, &a, &c, &ac, &d, "400").await;
    let app =
        faktura_server::banking::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/bank/movements/{}/split-candidates",
                    m.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let rest: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
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
    let args = serde_json::json!({"company_id":c,"id":m.id,"query":""});
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_split_candidates")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let json = serde_json::to_value(r).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(mcp, rest);
    let command =
        serde_json::to_value(split(&m, vec![allocation(&p, &a, &c, &d, "250.01").await])).unwrap();
    let mut args = command.clone();
    args["company_id"] = c.clone().into();
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_execute")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let json = serde_json::to_value(r).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/bank/commands"))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1")
                .body(Body::from(command.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let replay: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(mcp, replay);
    assert_eq!(replay["value"]["remaining"], "149.99");
    client.cancel().await.unwrap();
    task.await.unwrap();
}

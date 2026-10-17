use super::*;
use banking::{BatchInput, BatchPreview};
fn input(ids: &[String]) -> BatchInput {
    BatchInput {
        movement_ids: ids.to_vec(),
    }
}
fn confirm(ids: &[String], preview: &BatchPreview, request_id: &str) -> Command {
    Command {
        request_id: request_id.into(),
        action: Action::BatchMatch {
            movement_ids: ids.to_vec(),
            fingerprint: preview.fingerprint.clone(),
        },
    }
}
pub(super) async fn for_invoice(
    s: &PgBank,
    a: &str,
    c: &str,
    ac: &Account,
    d: &draft::Draft,
    value: &str,
) -> Movement {
    match run(
        s,
        a,
        c,
        Action::Manual {
            account_id: ac.id.clone(),
            data: MovementData {
                external_id: "".into(),
                booked_on: d.data.issued_on.unwrap(),
                amount: value.into(),
                currency: "CZK".into(),
                variable_symbol: d.data.payment.variable_symbol.clone(),
                counterparty: "Synthetic customer".into(),
                description: "Batch matching fixture".into(),
            },
        },
    )
    .await
    {
        Reply::Movement(m) => m,
        _ => panic!(),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn batch_new_existing_outgoing_and_replay(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let m1 = for_invoice(&bank, &a, &c, &ac, &d1, "300").await;
    let current = PgPayment(p.clone()).get(&a, &c, &d2.id).await.unwrap();
    payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d2.id,
        payment::Register {
            request_id: key(),
            revision: current.summary.revision,
            payment: PaymentInput {
                amount: "100".into(),
                paid_on: d2.data.issued_on.unwrap(),
                method: "bank_transfer".into(),
                note: "Existing".into(),
            },
        },
    )
    .await
    .unwrap();
    let m2 = for_invoice(&bank, &a, &c, &ac, &d2, "100").await;
    let received = PgReceived(p.clone())
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
    let m3 = movement(&bank, &a, &c, &ac, "-121").await;
    let skipped = movement(&bank, &a, &c, &ac, "17").await;
    let ids = vec![
        m1.id.clone(),
        m2.id.clone(),
        m3.id.clone(),
        skipped.id.clone(),
    ];
    let report = bank.batch_preview(&a, &c, input(&ids)).await.unwrap();
    assert_eq!(report.ready_count, 3);
    assert!(PgPayment(p.clone())
        .get(&a, &c, &d1.id)
        .await
        .unwrap()
        .items
        .is_empty());
    let request = key();
    let one = bank
        .execute(&a, &c, confirm(&ids, &report, &request))
        .await
        .unwrap();
    let two = bank
        .execute(&a, &c, confirm(&ids, &report, &request))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(one).unwrap(),
        serde_json::to_value(two).unwrap()
    );
    let m1 = bank.get(&a, &c, &m1.id).await.unwrap();
    assert!(m1.matches[0].created_payment);
    let m2 = bank.get(&a, &c, &m2.id).await.unwrap();
    assert!(!m2.matches[0].created_payment);
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d1.id)
            .await
            .unwrap()
            .summary
            .paid,
        "300.00"
    );
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d2.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        PgReceived(p.clone())
            .get(&a, &c, &received.id)
            .await
            .unwrap()
            .remaining,
        "0.00"
    );
    assert!(bank
        .get(&a, &c, &skipped.id)
        .await
        .unwrap()
        .matches
        .is_empty());
    unlink(&bank, &a, &c, &m1).await;
    unlink(&bank, &a, &c, &m2).await;
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d1.id)
            .await
            .unwrap()
            .summary
            .paid,
        "0.00"
    );
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d2.id)
            .await
            .unwrap()
            .summary
            .paid,
        "100.00"
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn batch_skips_competing_movements_outside_page_and_partial(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let partial = for_invoice(&bank, &a, &c, &ac, &d, "100").await;
    let r = bank
        .batch_preview(&a, &c, input(&[partial.id]))
        .await
        .unwrap();
    assert_eq!(r.items[0].status, "partial");
    let m1 = for_invoice(&bank, &a, &c, &ac, &d, "300").await;
    let ids = vec![m1.id.clone()];
    let before = bank.batch_preview(&a, &c, input(&ids)).await.unwrap();
    assert_eq!(before.ready_count, 1);
    let _outside = for_invoice(&bank, &a, &c, &ac, &d, "300").await;
    let report = bank.batch_preview(&a, &c, input(&ids)).await.unwrap();
    assert_eq!(report.items[0].status, "competing_movement");
    assert_eq!(report.ready_count, 0);
    assert!(matches!(
        bank.execute(&a, &c, confirm(&ids, &before, &key())).await,
        Err(banking::Error::Conflict)
    ));
    assert!(bank
        .execute(&a, &c, confirm(&ids, &report, &key()))
        .await
        .is_err());
    assert!(PgPayment(p.clone())
        .get(&a, &c, &d.id)
        .await
        .unwrap()
        .items
        .is_empty());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn batch_stale_scope_roles_and_validation(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = for_invoice(&bank, &a, &c, &ac, &d, "300").await;
    let ids = vec![m.id.clone()];
    let report = bank.batch_preview(&a, &c, input(&ids)).await.unwrap();
    assert!(bank.batch_preview(&a, &other, input(&ids)).await.is_err());
    assert!(bank
        .execute(&a, &other, confirm(&ids, &report, &key()))
        .await
        .is_err());
    assert!(bank.batch_preview(&a, &c, input(&[])).await.is_err());
    assert!(bank
        .batch_preview(&a, &c, input(&vec![m.id.clone(); 26]))
        .await
        .is_err());
    assert!(bank
        .batch_preview(&a, &c, input(&[m.id.clone(), format!("0{}", m.id)]))
        .await
        .is_err());
    let mut mixed = ids.clone();
    mixed.push("9999999".into());
    assert!(bank.batch_preview(&a, &c, input(&mixed)).await.is_err());
    let current = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: current.summary.revision,
            payment: PaymentInput {
                amount: "10".into(),
                paid_on: d.data.issued_on.unwrap(),
                method: "cash".into(),
                note: "".into(),
            },
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        bank.execute(&a, &c, confirm(&ids, &report, &key())).await,
        Err(banking::Error::Conflict)
    ));
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(bank.batch_preview(&a, &c, input(&ids)).await.is_ok());
    assert!(bank
        .execute(&a, &c, confirm(&ids, &report, &key()))
        .await
        .is_err());
    assert!(bank.get(&a, &c, &m.id).await.unwrap().matches.is_empty());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn batch_failure_rolls_back_first_payment_and_concurrent_confirmation(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let m1 = for_invoice(&bank, &a, &c, &ac, &d1, "300").await;
    let m2 = for_invoice(&bank, &a, &c, &ac, &d2, "300").await;
    let ids = vec![m1.id.clone(), m2.id.clone()];
    let report = bank.batch_preview(&a, &c, input(&ids)).await.unwrap();
    assert_eq!(report.ready_count, 2);
    let second = d2.id.parse::<i64>().unwrap();
    sqlx::raw_sql(&format!("CREATE FUNCTION batch_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.invoice_id={second} THEN RAISE EXCEPTION 'synthetic failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER batch_failure BEFORE INSERT ON invoice_payment FOR EACH ROW EXECUTE FUNCTION batch_failure();")).execute(&p).await.unwrap();
    let request = key();
    assert!(bank
        .execute(&a, &c, confirm(&ids, &report, &request))
        .await
        .is_err());
    assert!(PgPayment(p.clone())
        .get(&a, &c, &d1.id)
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(bank.get(&a, &c, &m1.id).await.unwrap().matches.is_empty());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM bank_command WHERE request_id=$1::text::uuid")
            .bind(&request)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(count, 0);
    sqlx::raw_sql("DROP TRIGGER batch_failure ON invoice_payment; DROP FUNCTION batch_failure();")
        .execute(&p)
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        bank.execute(&a, &c, confirm(&ids, &report, &request)),
        bank.execute(&a, &c, confirm(&ids, &report, &key()))
    );
    assert_eq!(u8::from(one.is_ok()) + u8::from(two.is_ok()), 1);
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d1.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d2.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn batch_rest_mcp_preview_execute_and_replay(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = for_invoice(&bank, &a, &c, &ac, &d, "300").await;
    let app =
        faktura_server::banking::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let ids = vec![m.id.clone()];
    let request = serde_json::to_value(input(&ids)).unwrap();
    let http_request = |path: &str, body: serde_json::Value, origin: bool| {
        let mut r = Request::builder()
            .method("POST")
            .uri(format!("/api/companies/{c}/bank/{path}"))
            .header("cookie", format!("faktura_session={token}"))
            .header("content-type", "application/json");
        if origin {
            r = r
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1");
        }
        r.body(Body::from(body.to_string())).unwrap()
    };
    let denied = app
        .clone()
        .oneshot(http_request("batch-preview", request.clone(), false))
        .await
        .unwrap();
    assert_eq!(denied.status(), 401);
    let r = app
        .clone()
        .oneshot(http_request("batch-preview", request.clone(), true))
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let preview: BatchPreview =
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(preview.ready_count, 1);
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
    let mut args = request;
    args["company_id"] = c.clone().into();
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_batch_preview")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let json = serde_json::to_value(r).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(mcp, serde_json::to_value(&preview).unwrap());
    let command = serde_json::to_value(confirm(&ids, &preview, &key())).unwrap();
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
    let executed: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    let r = app
        .oneshot(http_request("commands", command, true))
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let replay: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(executed, replay);
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

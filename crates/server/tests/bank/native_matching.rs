use super::batch_tests::for_invoice;
use super::foreign_tests::foreign_invoice;
use super::*;
pub(super) async fn eur(s: &PgBank, a: &str, c: &str) -> Account {
    let Reply::Account(ac) = run(
        s,
        a,
        c,
        Action::Account {
            id: None,
            revision: None,
            archived: false,
            data: AccountData {
                name: "EUR".into(),
                iban: "CZ6508000000192000145399".into(),
                currency: "EUR".into(),
            },
        },
    )
    .await
    else {
        panic!()
    };
    ac
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
async fn native_new_payment_replay_concurrency_currency_and_scope(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = eur(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let m = for_invoice(&s, &a, &c, &ac, &d, "100").await;
    let czk = issued_case(&p, &a, &c, 30, false).await;
    let choices = s.candidates(&a, &c, &m.id, "").await.unwrap();
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].document_id, d.id);
    assert_eq!(s.suggestions(&a, &c, &m.id).await.unwrap().len(), 1);
    assert!(s.candidates(&a, &other, &m.id, "").await.is_err());
    let mut wrong = match_cmd(&m, &choices[0]);
    if let Action::Match { document_id, .. } = &mut wrong.action {
        *document_id = czk.id;
    }
    assert!(s.execute(&a, &c, wrong).await.is_err());
    let command = match_cmd(&m, &choices[0]);
    let saved = serde_json::to_value(&command).unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, command),
        s.execute(&a, &c, match_cmd(&m, &choices[0]))
    );
    assert_ne!(one.is_ok(), two.is_ok());
    let matched = result(if let Ok(v) = one { v } else { two.unwrap() });
    assert_eq!(matched.allocated, "100.00");
    assert!(matched.matches[0].foreign.is_none());
    let paid = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    assert_eq!(paid.summary.outstanding, "0.00");
    assert_eq!(paid.items.len(), 1);
    // Repeating the actual winning request is covered by the shared bank regressions.
    // A losing request remains a conflict, never a second payment.
    let _ = s
        .execute(&a, &c, serde_json::from_value(saved).unwrap())
        .await;
    let report = s
        .foreign_report(
            &a,
            &c,
            banking::ForeignReportQuery {
                from: "2026-10-01".parse().unwrap(),
                to: "2026-10-31".parse().unwrap(),
                page: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(report.total, 0);
    let free = unlink(&s, &a, &c, &matched).await;
    assert_eq!(free.remaining, "100.00");
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .summary
            .outstanding,
        "100.00"
    );
    let candidate = s.candidates(&a, &c, &m.id, "").await.unwrap().remove(0);
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s
        .execute(&a, &c, match_cmd(&free, &candidate))
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_existing_payment_split_and_detachment(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = eur(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let before = payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: 0,
            payment: PaymentInput {
                amount: "100".into(),
                paid_on: "2026-10-01".parse().unwrap(),
                method: "bank_transfer".into(),
                note: "Recorded EUR".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = for_invoice(&s, &a, &c, &ac, &d, "150").await;
    let choices = s.split_candidates(&a, &c, &m.id, "").await.unwrap();
    assert_eq!(choices.len(), 1);
    let x = &choices[0];
    assert!(x.payment_id.is_some());
    let cmd = Command {
        request_id: key(),
        action: Action::Split {
            movement_id: m.id.clone(),
            revision: m.revision,
            allocations: vec![banking::Allocation {
                target_kind: x.target_kind.clone(),
                document_id: x.document_id.clone(),
                target_revision: x.revision,
                payment_id: x.payment_id.clone(),
                amount: "100".into(),
            }],
        },
    };
    let retry = serde_json::to_value(&cmd).unwrap();
    let matched = result(s.execute(&a, &c, cmd).await.unwrap());
    assert!(!matched.matches[0].created_payment);
    assert_eq!(matched.remaining, "50.00");
    assert_eq!(
        result(
            s.execute(&a, &c, serde_json::from_value(retry).unwrap())
                .await
                .unwrap()
        )
        .revision,
        matched.revision
    );
    let competitor = for_invoice(&s, &a, &c, &ac, &d, "100").await;
    assert!(s
        .candidates(&a, &c, &competitor.id, "")
        .await
        .unwrap()
        .is_empty());
    unlink(&s, &a, &c, &matched).await;
    let after = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    assert_eq!(
        s.candidates(&a, &c, &competitor.id, "")
            .await
            .unwrap()
            .len(),
        1
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_outgoing_new_then_existing_payment(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = eur(&s, &a, &c).await;
    let mut data = data();
    data.currency = "EUR".into();
    data.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "25".into(),
        units: "1".into(),
    });
    data.rounding = "0".into();
    data.lines[0].net = "100".into();
    data.lines[0].vat = "0".into();
    let store = PgReceived(p.clone());
    let d = store
        .create(
            &a,
            &c,
            received::Create {
                request_id: key(),
                data,
            },
        )
        .await
        .unwrap();
    let m = movement(&s, &a, &c, &ac, "-100").await;
    let x = s.candidates(&a, &c, &m.id, "").await.unwrap().remove(0);
    let matched = result(s.execute(&a, &c, match_cmd(&m, &x)).await.unwrap());
    assert_eq!(store.get(&a, &c, &d.id).await.unwrap().remaining, "0.00");
    let free = unlink(&s, &a, &c, &matched).await;
    let d = store.get(&a, &c, &d.id).await.unwrap();
    assert_eq!(d.remaining, "100.00");
    store
        .command(
            &a,
            &c,
            &d.id,
            received::Command {
                request_id: key(),
                revision: d.revision,
                action: received::Action::Pay {
                    payment: PaymentInput {
                        amount: "100".into(),
                        paid_on: "2026-10-01".parse().unwrap(),
                        method: "bank_transfer".into(),
                        note: "Existing EUR purchase".into(),
                    },
                },
            },
        )
        .await
        .unwrap();
    let x = s.candidates(&a, &c, &m.id, "").await.unwrap().remove(0);
    assert!(x.payment_id.is_some());
    let matched = result(s.execute(&a, &c, match_cmd(&free, &x)).await.unwrap());
    assert!(!matched.matches.last().unwrap().created_payment);
    unlink(&s, &a, &c, &matched).await;
    assert_eq!(store.get(&a, &c, &d.id).await.unwrap().remaining, "0.00");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_rest_mcp_existing_payment_parity(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = eur(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: 0,
            payment: PaymentInput {
                amount: "100".into(),
                paid_on: "2026-10-01".parse().unwrap(),
                method: "bank_transfer".into(),
                note: "Native REST/MCP".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = for_invoice(&s, &a, &c, &ac, &d, "100").await;
    let app =
        faktura_server::banking::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/bank/movements/{}/candidates",
                    m.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let http: serde_json::Value =
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
    let result = client
        .call_tool(
            CallToolRequestParams::new("bank_candidates").with_arguments(
                serde_json::json!({"company_id":c,"id":m.id,"query":""})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let mcp: serde_json::Value =
        serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(http, mcp);
    let choices: Vec<Candidate> = serde_json::from_value(http).unwrap();
    assert_eq!(choices.len(), 1);
    assert!(choices[0].payment_id.is_some());
    let mut args = serde_json::to_value(match_cmd(&m, &choices[0])).unwrap();
    args["company_id"] = c.clone().into();
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_execute")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let result: serde_json::Value =
        serde_json::from_str(&r.content[0].as_text().unwrap().text).unwrap();
    let r = app
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
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(http, result["value"]);
    assert_eq!(http["matches"][0]["created_payment"], false);
    client.cancel().await.unwrap();
    task.await.unwrap();
}

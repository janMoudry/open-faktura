use super::*;
use faktura_domain::exchange::ExchangeRate;
async fn usd(s: &PgBank, a: &str, c: &str) -> Account {
    let Reply::Account(v) = run(
        s,
        a,
        c,
        Action::Account {
            id: None,
            revision: None,
            archived: false,
            data: AccountData {
                name: "USD".into(),
                iban: "CZ6508000000192000145399".into(),
                currency: "USD".into(),
            },
        },
    )
    .await
    else {
        panic!()
    };
    v
}
fn command(
    m: &Movement,
    d: &banking::ForeignCandidate,
    bank: &str,
    paid: &str,
    rate: &str,
) -> Command {
    Command {
        request_id: key(),
        action: Action::ForeignMatch {
            bank_rate: Some(ExchangeRate {
                rate: rate.into(),
                units: "1".into(),
            }),
            payment_id: d.payment_id.clone(),
            movement_id: m.id.clone(),
            revision: m.revision,
            target_kind: d.target_kind.clone(),
            document_id: d.document_id.clone(),
            target_revision: d.revision,
            bank_amount: bank.into(),
            document_amount: paid.into(),
        },
    }
}
fn result(r: Reply) -> Movement {
    let Reply::Movement(m) = r else { panic!() };
    m
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn cross_incoming_validation_replay_scope_report_and_reverse(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = usd(&s, &a, &c).await;
    let d = super::foreign_tests::foreign_invoice(&p, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "120").await;
    let choices = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    let candidate = &choices[0];
    assert_eq!(candidate.currency, "EUR");
    for rate in [
        None,
        Some(ExchangeRate {
            rate: "0".into(),
            units: "1".into(),
        }),
        Some(ExchangeRate {
            rate: "22.5".into(),
            units: "0".into(),
        }),
    ] {
        let mut cmd = command(&m, candidate, "110", "100", "22.5");
        if let Action::ForeignMatch { bank_rate, .. } = &mut cmd.action {
            *bank_rate = rate;
        }
        assert!(s.execute(&a, &c, cmd).await.is_err());
    }
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .items
            .len(),
        0
    );
    let cmd = serde_json::to_value(command(&m, candidate, "110", "100", "22.5")).unwrap();
    assert!(s
        .execute(&a, &other, serde_json::from_value(cmd.clone()).unwrap())
        .await
        .is_err());
    let saved = result(
        s.execute(&a, &c, serde_json::from_value(cmd.clone()).unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(saved.remaining, "10.00");
    let f = saved.matches[0].foreign.as_ref().unwrap();
    assert_eq!(f.bank_currency, "USD");
    assert_eq!(f.bank_value_czk.as_deref(), Some("2475.00"));
    assert_eq!(f.book_value_czk, "2500.00");
    assert_eq!(f.difference_czk, "-25.00");
    assert_eq!(
        result(
            s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
                .await
                .unwrap()
        )
        .revision,
        saved.revision
    );
    let report = s
        .foreign_report(
            &a,
            &c,
            banking::ForeignReportQuery {
                from: "2026-10-01".parse().unwrap(),
                to: "2026-10-01".parse().unwrap(),
                page: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(report.losses, "25.00");
    assert_eq!(report.items[0].bank_amount, "110.00");
    assert_eq!(report.items[0].settlement.bank_currency, "USD");
    unlink(&s, &a, &c, &saved).await;
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .summary
            .outstanding,
        "100.00"
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn cross_outgoing_czk_existing_payment_and_native_exclusion(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = super::native_matching::eur(&s, &a, &c).await;
    let mut input = data();
    input.rounding = "0".into();
    input.lines[0].net = "100".into();
    input.lines[0].vat = "0".into();
    let received = PgReceived(p.clone());
    let d = received
        .create(
            &a,
            &c,
            received::Create {
                request_id: key(),
                data: input,
            },
        )
        .await
        .unwrap();
    let m = movement(&s, &a, &c, &ac, "-5").await;
    let candidate = s
        .foreign_candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .remove(0);
    assert_eq!(candidate.currency, "CZK");
    let saved = result(
        s.execute(&a, &c, command(&m, &candidate, "4", "100", "24.9"))
            .await
            .unwrap(),
    );
    assert_eq!(saved.remaining, "1.00");
    assert_eq!(
        saved.matches[0].foreign.as_ref().unwrap().difference_czk,
        "0.40"
    );
    unlink(&s, &a, &c, &saved).await;
    let d = received.get(&a, &c, &d.id).await.unwrap();
    received
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
                        note: "Existing CZK".into(),
                    },
                },
            },
        )
        .await
        .unwrap();
    let m = movement(&s, &a, &c, &ac, "-4").await;
    let candidate = s
        .foreign_candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.payment_id.is_some())
        .unwrap();
    let saved = result(
        s.execute(&a, &c, command(&m, &candidate, "4", "100", "24.9"))
            .await
            .unwrap(),
    );
    assert!(!saved.matches[0].created_payment);
    unlink(&s, &a, &c, &saved).await;
    assert_eq!(received.get(&a, &c, &d.id).await.unwrap().remaining, "0.00");
    super::foreign_tests::foreign_invoice(&p, &a, &c).await;
    let incoming = movement(&s, &a, &c, &ac, "100").await;
    assert!(s
        .foreign_candidates(&a, &c, &incoming.id, "")
        .await
        .unwrap()
        .is_empty());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn cross_rest_mcp_and_viewer(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = usd(&bank, &a, &c).await;
    let d = super::foreign_tests::foreign_invoice(&p, &a, &c).await;
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
                note: "REST/MCP existing FX".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = movement(&bank, &a, &c, &ac, "110").await;
    let app =
        faktura_server::banking::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/bank/movements/{}/foreign-candidates",
                    m.id
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let choices: Vec<banking::ForeignCandidate> =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
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
    let reply = client
        .call_tool(
            CallToolRequestParams::new("bank_foreign_candidates").with_arguments(
                serde_json::json!({"company_id":c,"id":m.id,"query":""})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert!(!reply.is_error.unwrap_or(false));
    let mut payload = serde_json::to_value(command(&m, &choices[0], "110", "100", "22.5")).unwrap();
    payload["company_id"] = c.clone().into();
    payload["payment_id"] = choices[0].payment_id.clone().unwrap().into();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    let denied = client
        .call_tool(
            CallToolRequestParams::new("bank_execute")
                .with_arguments(payload.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert!(denied.is_error.unwrap_or(false));
    sqlx::query("UPDATE membership SET role='owner' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    let reply = client
        .call_tool(
            CallToolRequestParams::new("bank_execute")
                .with_arguments(payload.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert!(!reply.is_error.unwrap_or(false));
    assert_eq!(
        bank.get(&a, &c, &m.id).await.unwrap().matches[0]
            .foreign
            .as_ref()
            .unwrap()
            .difference_czk,
        "-25.00"
    );
    let report = client
        .call_tool(
            CallToolRequestParams::new("bank_foreign_report").with_arguments(
                serde_json::json!({"company_id":c,"from":"2026-10-01","to":"2026-10-01","page":1})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert!(!report.is_error.unwrap_or(false));
    let text = report.content[0].as_text().unwrap().text.clone();
    let mcp: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(mcp["losses"], "25.00");
    // The same query remains readable by a viewer and does not change payments.
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    let (_, token) = auth::login(
        &PgAuth::new(p.clone(), None),
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/bank/foreign-report?from=2026-10-01&to=2026-10-01&page=1"
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(http, mcp);
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn cross_upgrade054_preserves_legacy_snapshot(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610180055)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    super::foreign_tests::foreign_invoice(&p, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "2490").await;
    let candidate = s
        .foreign_candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .remove(0);
    let mut cmd = command(&m, &candidate, "2490", "100", "1");
    if let Action::ForeignMatch { bank_rate, .. } = &mut cmd.action {
        *bank_rate = None;
    }
    let cmd = serde_json::to_value(cmd).unwrap();
    assert!(cmd.get("bank_rate").is_none());
    s.execute(&a, &c, serde_json::from_value(cmd.clone()).unwrap())
        .await
        .unwrap();
    sqlx::query("UPDATE bank_match SET foreign_settlement=foreign_settlement-'bank_currency'-'bank_rate'-'bank_value_czk' WHERE company_id=$1::text::bigint")
        .bind(&c).execute(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let fresh = PgPool::connect_with((*p.connect_options()).clone())
        .await
        .unwrap();
    let s = PgBank(fresh.clone());
    let saved = s.get(&a, &c, &m.id).await.unwrap();
    let f = saved.matches[0].foreign.as_ref().unwrap();
    assert_eq!(f.bank_currency, "CZK");
    assert!(f.bank_rate.is_none());
    assert_eq!(f.difference_czk, "-10.00");
    assert_eq!(
        result(
            s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
                .await
                .unwrap()
        )
        .matches
        .len(),
        1
    );
    fresh.close().await;
}

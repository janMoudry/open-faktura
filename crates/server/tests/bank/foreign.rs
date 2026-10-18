use super::*;
use faktura_domain::exchange::ExchangeRate;
fn rate() -> ExchangeRate {
    ExchangeRate {
        rate: "25".into(),
        units: "1".into(),
    }
}
pub(super) async fn foreign_invoice(p: &PgPool, a: &str, c: &str) -> draft::Draft {
    let mut d: DraftData = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/invoice-draft.json"
    ))
    .unwrap();
    d.issued_on = Some("2026-10-01".parse().unwrap());
    d.due_on = d.issued_on;
    d.tax_on = None;
    d.supplier.registration_number = "12345678".into();
    d.payment.method = "bank_transfer".into();
    d.payment.bank_account = "12345678/0100".into();
    d.calculation.currency = "EUR".into();
    d.calculation.exchange_rate = Some(rate());
    d.calculation.tax_mode = TaxMode::NonPayer;
    d.calculation.rounding = faktura_domain::invoice::Rounding::None;
    d.calculation.discount_percent = "0".into();
    d.calculation.lines.truncate(1);
    d.calculation.lines[0].quantity = "1".into();
    d.calculation.lines[0].unit_price = "100".into();
    d.calculation.lines[0].vat_percent = "0".into();
    d.calculation.lines[0].discount_percent = "0".into();
    let s = PgDraft(p.clone());
    let d = draft::create(
        &s,
        a,
        c,
        draft::Create {
            request_id: key(),
            data: d,
        },
    )
    .await
    .unwrap();
    // This helper also seeds pre-059/082 upgrade databases. Do not call today's
    // issuing implementation against a historical schema.
    let modern:bool=sqlx::query_scalar("SELECT to_regclass('invoice_trash') IS NOT NULL AND to_regclass('company_document_style') IS NOT NULL").fetch_one(p).await.unwrap();
    if !modern {
        let number = format!("LEGACY-{}", d.id);
        sqlx::query("UPDATE invoice_draft SET number=$3,issued_at=now(),issue_request_id=$4::text::uuid,issue_actor_id=$5::text::bigint,issue_revision=revision,revision=revision+1 WHERE company_id=$1::text::bigint AND id=$2::text::bigint").bind(c).bind(&d.id).bind(&number).bind(key()).bind(a).execute(p).await.unwrap();
        let mut historical = d;
        historical.number = Some(number);
        historical.revision += 1;
        historical.issued_at = Some(sqlx::types::chrono::Utc::now().to_rfc3339());
        return historical;
    }
    draft::issue(
        &s,
        a,
        c,
        &d.id,
        draft::Issue {
            request_id: key(),
            revision: d.revision,
        },
    )
    .await
    .unwrap()
}
fn command(m: &Movement, d: &banking::ForeignCandidate, bank: &str, foreign: &str) -> Command {
    Command {
        request_id: key(),
        action: Action::ForeignMatch {
            bank_rate: None,
            payment_id: None,
            movement_id: m.id.clone(),
            revision: m.revision,
            target_kind: d.target_kind.clone(),
            document_id: d.document_id.clone(),
            target_revision: d.revision,
            bank_amount: bank.into(),
            document_amount: foreign.into(),
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
async fn foreign_incoming_snapshot_replay_validation_and_reversal(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "2500").await;
    assert!(s.candidates(&a, &c, &m.id, "").await.unwrap().is_empty());
    assert!(s.suggestions(&a, &c, &m.id).await.unwrap().is_empty());
    assert!(s.foreign_candidates(&a, &other, &m.id, "").await.is_err());
    let choices = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    assert_eq!(choices.len(), 1);
    let selected = &choices[0];
    assert_eq!(selected.outstanding, "100.00");
    for (bank, foreign) in [
        ("2501", "100"),
        ("2490", "101"),
        ("0", "100"),
        ("2490", "0.001"),
    ] {
        assert!(s
            .execute(&a, &c, command(&m, selected, bank, foreign))
            .await
            .is_err());
    }
    let cmd = serde_json::to_value(command(&m, selected, "2490", "100")).unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, serde_json::from_value(cmd.clone()).unwrap()),
        s.execute(&a, &c, serde_json::from_value(cmd.clone()).unwrap())
    );
    let m = result(one.unwrap());
    assert_eq!(result(two.unwrap()).revision, m.revision);
    assert_eq!(m.remaining, "10.00");
    let fx = m.matches[0].foreign.as_ref().unwrap();
    assert_eq!(fx.document_amount, "100.00");
    assert_eq!(fx.book_value_czk, "2500.00");
    assert_eq!(fx.difference_czk, "-10.00");
    let payments = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    assert_eq!(payments.summary.outstanding, "0.00");
    assert_eq!(payments.items.len(), 1);
    assert!(s
        .execute(&a, &c, command(&m, selected, "10", "1"))
        .await
        .is_err());
    let unlinked = result(
        run(
            &s,
            &a,
            &c,
            Action::Unmatch {
                movement_id: m.id.clone(),
                revision: m.revision,
                reason: "Synthetic correction".into(),
            },
        )
        .await,
    );
    assert_eq!(unlinked.remaining, "2500.00");
    assert!(unlinked.matches[0].detached_at.is_some());
    assert_eq!(
        unlinked.matches[0].foreign.as_ref().unwrap().difference_czk,
        "-10.00"
    );
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .summary
            .outstanding,
        "100.00"
    );
    let replay = result(
        s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(replay.remaining, "10.00");
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_outgoing_partial_gain_and_profile_scope(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let mut data = data();
    data.currency = "EUR".into();
    data.exchange_rate = Some(rate());
    data.rounding = "0".into();
    data.lines[0].net = "100".into();
    data.lines[0].vat = "0".into();
    let d = PgReceived(p.clone())
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
    let m = movement(&s, &a, &c, &ac, "-2490").await;
    let cs = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    assert_eq!(cs.len(), 1);
    let cmd = serde_json::to_value(command(&m, &cs[0], "1000", "40")).unwrap();
    assert!(s
        .execute(&a, &other, serde_json::from_value(cmd.clone()).unwrap())
        .await
        .is_err());
    let saved = result(
        s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(saved.remaining, "1490.00");
    assert_eq!(
        saved.matches[0].foreign.as_ref().unwrap().difference_czk,
        "0.00"
    );
    let cs = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    let saved = result(
        s.execute(&a, &c, command(&saved, &cs[0], "1490", "60"))
            .await
            .unwrap(),
    );
    assert_eq!(
        saved.matches[1].foreign.as_ref().unwrap().difference_czk,
        "10.00"
    );
    assert_eq!(
        PgReceived(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .remaining,
        "0.00"
    );
    run(
        &s,
        &a,
        &c,
        Action::Unmatch {
            movement_id: saved.id,
            revision: saved.revision,
            reason: "Synthetic reversal".into(),
        },
    )
    .await;
    assert_eq!(
        PgReceived(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .remaining,
        "100.00"
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_rest_mcp_and_viewer(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
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
                note: "REST/MCP existing FX".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = movement(&bank, &a, &c, &ac, "2490").await;
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
    let mut payload = serde_json::to_value(command(&m, &choices[0], "2490", "100")).unwrap();
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
        "-10.00"
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
    assert_eq!(mcp["losses"], "10.00");
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
async fn foreign_upgrade_051_preserves_czk_match(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610170052)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = super::batch_tests::for_invoice(&s, &a, &c, &ac, &d, "20").await;
    let cs = s.candidates(&a, &c, &m.id, "").await.unwrap();
    let cmd = serde_json::to_value(match_cmd(&m, &cs[0])).unwrap();
    let before = result(
        s.execute(&a, &c, serde_json::from_value(cmd.clone()).unwrap())
            .await
            .unwrap(),
    );
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let options = (*p.connect_options()).clone();
    p.close().await;
    let p = PgPool::connect_with(options).await.unwrap();
    let s = PgBank(p.clone());
    let after = s.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(after.matches[0].amount, before.matches[0].amount);
    assert!(after.matches[0].foreign.is_none());
    let replay = result(
        s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
            .await
            .unwrap(),
    );
    assert!(replay.matches[0].foreign.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_existing_incoming_no_duplicate_unmatch_and_competition(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let payments = payment::register(
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
                note: "Existing FX".into(),
            },
        },
    )
    .await
    .unwrap();
    let payment_id = payments.items[0].id.clone();
    let m = movement(&s, &a, &c, &ac, "2490").await;
    let competing = movement(&s, &a, &c, &ac, "2480").await;
    let choices = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].payment_id.as_deref(), Some(payment_id.as_str()));
    let mut value = serde_json::to_value(command(&m, &choices[0], "2490", "100")).unwrap();
    value["payment_id"] = payment_id.clone().into();
    let mut wrong = value.clone();
    wrong["document_amount"] = "99".into();
    assert!(s
        .execute(&a, &c, serde_json::from_value(wrong).unwrap())
        .await
        .is_err());
    assert!(s
        .execute(&a, &other, serde_json::from_value(value.clone()).unwrap())
        .await
        .is_err());
    let saved = result(
        s.execute(&a, &c, serde_json::from_value(value.clone()).unwrap())
            .await
            .unwrap(),
    );
    assert!(!saved.matches[0].created_payment);
    assert_eq!(saved.matches[0].payment_id, payment_id);
    let again = result(
        s.execute(&a, &c, serde_json::from_value(value).unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(again.revision, saved.revision);
    let mut duplicate =
        serde_json::to_value(command(&competing, &choices[0], "2480", "100")).unwrap();
    duplicate["payment_id"] = payment_id.clone().into();
    assert!(s
        .execute(&a, &c, serde_json::from_value(duplicate).unwrap())
        .await
        .is_err());
    assert!(s
        .foreign_candidates(&a, &c, &competing.id, "")
        .await
        .unwrap()
        .is_empty());
    let now = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    assert_eq!(now.items.len(), 1);
    assert_eq!(now.summary.revision, payments.summary.revision);
    assert!(payment::reverse(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        &payment_id,
        payment::Reverse {
            request_id: key(),
            revision: now.summary.revision,
            reason: "Blocked while matched".into(),
            replacement: None
        }
    )
    .await
    .is_err());
    run(
        &s,
        &a,
        &c,
        Action::Unmatch {
            movement_id: saved.id,
            revision: saved.revision,
            reason: "Disconnect only".into(),
        },
    )
    .await;
    let now = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    assert_eq!(now.items.len(), 1);
    assert!(now.items[0].reversed_at.is_none());
    assert_eq!(now.summary.outstanding, "0.00");
    assert_eq!(
        s.foreign_candidates(&a, &c, &competing.id, "")
            .await
            .unwrap()
            .len(),
        1
    );
    let choices = s
        .foreign_candidates(&a, &c, &competing.id, "")
        .await
        .unwrap();
    let fresh = s.get(&a, &c, &m.id).await.unwrap();
    let mut one = serde_json::to_value(command(&fresh, &choices[0], "2490", "100")).unwrap();
    one["payment_id"] = payment_id.clone().into();
    let mut two = serde_json::to_value(command(&competing, &choices[0], "2480", "100")).unwrap();
    two["payment_id"] = payment_id.into();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, serde_json::from_value(one).unwrap()),
        s.execute(&a, &c, serde_json::from_value(two).unwrap())
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_existing_outgoing_preserves_original_payment(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let mut data = data();
    data.currency = "EUR".into();
    data.exchange_rate = Some(rate());
    data.rounding = "0".into();
    data.lines[0].net = "100".into();
    data.lines[0].vat = "0".into();
    let rs = PgReceived(p.clone());
    let d = rs
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
    let payment_id = key();
    let paid = rs
        .command(
            &a,
            &c,
            &d.id,
            received::Command {
                request_id: payment_id.clone(),
                revision: d.revision,
                action: received::Action::Pay {
                    payment: PaymentInput {
                        amount: "40".into(),
                        paid_on: "2026-10-02".parse().unwrap(),
                        method: "bank_transfer".into(),
                        note: "Preserved original".into(),
                    },
                },
            },
        )
        .await
        .unwrap();
    let m = movement(&s, &a, &c, &ac, "-990").await;
    let choices = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    assert_eq!(choices.len(), 2);
    let existing = choices
        .iter()
        .find(|x| x.payment_id.as_deref() == Some(payment_id.as_str()))
        .unwrap();
    let mut cmd = serde_json::to_value(command(&m, existing, "990", "40")).unwrap();
    cmd["payment_id"] = payment_id.into();
    let saved = result(
        s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
            .await
            .unwrap(),
    );
    assert!(!saved.matches[0].created_payment);
    assert_eq!(
        saved.matches[0].foreign.as_ref().unwrap().difference_czk,
        "10.00"
    );
    run(
        &s,
        &a,
        &c,
        Action::Unmatch {
            movement_id: saved.id,
            revision: saved.revision,
            reason: "Detach only".into(),
        },
    )
    .await;
    let after = rs.get(&a, &c, &d.id).await.unwrap();
    assert_eq!(
        serde_json::to_value(after.payments).unwrap(),
        serde_json::to_value(paid.payments).unwrap()
    );
    assert_eq!(after.revision, paid.revision);
    assert_eq!(after.remaining, "60.00");
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn foreign_upgrade_052_keeps_new_payment_replay(p: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610170053)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "2490").await;
    let choices = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
    let cmd = serde_json::to_value(command(&m, &choices[0], "2490", "100")).unwrap();
    assert!(
        cmd.get("payment_id").is_none(),
        "Preserve the pre-053 idempotency input"
    );
    let before = result(
        s.execute(&a, &c, serde_json::from_value(cmd.clone()).unwrap())
            .await
            .unwrap(),
    );
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let options = (*p.connect_options()).clone();
    p.close().await;
    let p = PgPool::connect_with(options).await.unwrap();
    let s = PgBank(p.clone());
    let replay = result(
        s.execute(&a, &c, serde_json::from_value(cmd).unwrap())
            .await
            .unwrap(),
    );
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &d.id)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
}

fn report_query(from: &str, to: &str, page: i64) -> banking::ForeignReportQuery {
    banking::ForeignReportQuery {
        from: from.parse().unwrap(),
        to: to.parse().unwrap(),
        page: Some(page),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_report_totals_pages_period_scope_and_unmatch(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    foreign_invoice(&p, &a, &c).await;
    let mut m = movement(&s, &a, &c, &ac, "700").await;
    for i in 0..26 {
        let choices = s.foreign_candidates(&a, &c, &m.id, "").await.unwrap();
        let new = choices.iter().find(|x| x.payment_id.is_none()).unwrap();
        m = result(
            s.execute(
                &a,
                &c,
                command(&m, new, if i % 2 == 0 { "24" } else { "26" }, "1"),
            )
            .await
            .unwrap(),
        );
    }
    let first = s
        .foreign_report(&a, &c, report_query("2026-10-01", "2026-10-01", 1))
        .await
        .unwrap();
    assert_eq!(first.gains, "13.00");
    assert_eq!(first.losses, "13.00");
    assert_eq!(first.net, "0.00");
    assert_eq!(first.total, 26);
    assert_eq!(first.items.len(), 25);
    let second = s
        .foreign_report(&a, &c, report_query("2026-10-01", "2026-10-01", 2))
        .await
        .unwrap();
    assert_eq!(second.total, 26);
    assert_eq!(second.gains, first.gains);
    assert_eq!(second.items.len(), 1);
    assert!(!first.items.iter().any(|x| x.id == second.items[0].id));
    for (profile, from, to) in [
        (&other, "2026-10-01", "2026-10-01"),
        (&c, "2026-10-02", "2026-10-02"),
    ] {
        let empty = s
            .foreign_report(&a, profile, report_query(from, to, 1))
            .await
            .unwrap();
        assert_eq!(empty.total, 0);
        assert_eq!(empty.net, "0.00");
    }
    assert!(s
        .foreign_report(&a, &c, report_query("2026-10-02", "2026-10-01", 1))
        .await
        .is_err());
    assert!(s
        .foreign_report(&a, &c, report_query("2026-10-01", "2026-10-01", 0))
        .await
        .is_err());
    run(
        &s,
        &a,
        &c,
        Action::Unmatch {
            movement_id: m.id,
            revision: m.revision,
            reason: "Report reversal".into(),
        },
    )
    .await;
    let empty = s
        .foreign_report(&a, &c, report_query("2026-10-01", "2026-10-01", 1))
        .await
        .unwrap();
    assert_eq!(empty.total, 0);
    assert_eq!(empty.gains, "0.00");
    assert_eq!(empty.losses, "0.00");
}

use super::batch_tests::for_invoice;
use super::*;
use faktura_application::bank_connection::{self as bc, ConnectionStore};
use faktura_infrastructure::bank_connection::PgConnection;
pub(super) async fn enable(p: &PgPool, a: &str, c: &str, ac: &Account) {
    let s = PgConnection::new(p.clone());
    let v = s.status(a, c, &ac.id).await.unwrap();
    assert!(!v.matching_enabled);
    s.execute(
        a,
        c,
        bc::Command {
            account_id: ac.id.clone(),
            revision: v.revision,
            request_id: key(),
            action: bc::Action::Matching {
                enabled: true,
                include_treasury: false,
            },
        },
    )
    .await
    .unwrap();
    due(p).await;
}
pub(super) async fn due(p: &PgPool) {
    sqlx::query("UPDATE bank_connection SET matching_next_at=now()-interval '1 minute' WHERE matching_enabled").execute(p).await.unwrap();
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn exact_concurrent_and_manual_detachment(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = for_invoice(&bank, &a, &c, &ac, &d, "300").await;
    assert!(!bank.process_matching_one().await.unwrap());
    let s = PgConnection::new(p.clone());
    assert!(s
        .execute(
            &a,
            &other,
            bc::Command {
                account_id: ac.id.clone(),
                revision: 0,
                request_id: key(),
                action: bc::Action::Matching {
                    enabled: true,
                    include_treasury: false
                }
            }
        )
        .await
        .is_err());
    enable(&p, &a, &c, &ac).await;
    let (one, two) = tokio::join!(bank.process_matching_one(), bank.process_matching_one());
    one.unwrap();
    two.unwrap();
    let v = s.status(&a, &c, &ac.id).await.unwrap();
    assert!(!v.configured);
    assert!(!v.schedule_enabled);
    assert_eq!(v.matching_last.unwrap().matched, 1);
    let paid = PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap();
    assert_eq!(paid.items.len(), 1);
    let linked = bank.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(linked.matches.len(), 1);
    unlink(&bank, &a, &c, &linked).await;
    due(&p).await;
    bank.process_matching_one().await.unwrap();
    let history = bank.get(&a, &c, &m.id).await.unwrap().matches;
    assert_eq!(history.len(), 1);
    assert!(history[0].detached_at.is_some());
    assert_eq!(
        s.status(&a, &c, &ac.id)
            .await
            .unwrap()
            .matching_last
            .unwrap()
            .checked,
        0
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn existing_payment_is_linked_once_and_permission_loss_pauses(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d = issued_case(&p, &a, &c, 20, false).await;
    let payments = PgPayment(p.clone());
    let current = payments.get(&a, &c, &d.id).await.unwrap();
    payment::register(
        &payments,
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: key(),
            revision: current.summary.revision,
            payment: PaymentInput {
                amount: "100".into(),
                paid_on: d.data.issued_on.unwrap(),
                method: "bank_transfer".into(),
                note: "Synthetic".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = for_invoice(&bank, &a, &c, &ac, &d, "100").await;
    enable(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    assert_eq!(payments.get(&a, &c, &d.id).await.unwrap().items.len(), 1);
    assert_eq!(bank.get(&a, &c, &m.id).await.unwrap().matches.len(), 1);
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    due(&p).await;
    bank.process_matching_one().await.unwrap();
    let v = PgConnection::new(p.clone())
        .status(&a, &c, &ac.id)
        .await
        .unwrap();
    assert!(!v.matching_enabled);
    assert_eq!(v.matching_error.as_deref(), Some("matching_permission"));
    assert!(PgConnection::new(p.clone())
        .execute(
            &a,
            &c,
            bc::Command {
                account_id: ac.id,
                revision: v.revision,
                request_id: key(),
                action: bc::Action::Matching {
                    enabled: true,
                    include_treasury: false
                }
            }
        )
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn cursor_skips_uncertain_rows_and_checks_competitors_outside_batch(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let contested = for_invoice(&bank, &a, &c, &ac, &d1, "300").await;
    for _ in 0..24 {
        movement(&bank, &a, &c, &ac, "17").await;
    }
    for_invoice(&bank, &a, &c, &ac, &d1, "300").await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let exact = for_invoice(&bank, &a, &c, &ac, &d2, "300").await;
    enable(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    let s = PgConnection::new(p.clone());
    let run = s
        .status(&a, &c, &ac.id)
        .await
        .unwrap()
        .matching_last
        .unwrap();
    assert_eq!(run.checked, 25);
    assert_eq!(run.matched, 0);
    due(&p).await;
    bank.process_matching_one().await.unwrap();
    assert!(bank
        .get(&a, &c, &contested.id)
        .await
        .unwrap()
        .matches
        .is_empty());
    assert_eq!(bank.get(&a, &c, &exact.id).await.unwrap().matches.len(), 1);
    assert_eq!(
        s.status(&a, &c, &ac.id)
            .await
            .unwrap()
            .matching_last
            .unwrap()
            .checked,
        2
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn failed_batch_rolls_back_and_retries(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let d1 = issued_case(&p, &a, &c, 20, false).await;
    let d2 = issued_case(&p, &a, &c, 20, false).await;
    let m1 = for_invoice(&bank, &a, &c, &ac, &d1, "300").await;
    for_invoice(&bank, &a, &c, &ac, &d2, "300").await;
    let second = d2.id.parse::<i64>().unwrap();
    sqlx::raw_sql(&format!("CREATE FUNCTION auto_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.invoice_id={second} THEN RAISE EXCEPTION 'synthetic failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER auto_failure BEFORE INSERT ON invoice_payment FOR EACH ROW EXECUTE FUNCTION auto_failure();")).execute(&p).await.unwrap();
    enable(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    assert!(PgPayment(p.clone())
        .get(&a, &c, &d1.id)
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(bank.get(&a, &c, &m1.id).await.unwrap().matches.is_empty());
    let s = PgConnection::new(p.clone());
    let v = s.status(&a, &c, &ac.id).await.unwrap();
    assert_eq!(v.matching_error.as_deref(), Some("matching_failed"));
    assert!(v.matching_last.is_none());
    sqlx::raw_sql("DROP TRIGGER auto_failure ON invoice_payment; DROP FUNCTION auto_failure();")
        .execute(&p)
        .await
        .unwrap();
    due(&p).await;
    bank.process_matching_one().await.unwrap();
    let v = s.status(&a, &c, &ac.id).await.unwrap();
    assert!(v.matching_error.is_none());
    assert_eq!(v.matching_last.unwrap().matched, 2);
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade034_preserves_download_schedule_but_matching_is_off(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140035)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let ac = account(&PgBank(p.clone()), &a, &c).await;
    sqlx::query("INSERT INTO bank_connection(company_id,account_id,provider,secret,revision,schedule_enabled,schedule_actor_id,next_run_at) VALUES($1::text::bigint,$2::text::bigint,'fio',$3,4,true,$4::text::bigint,now()+interval '1 hour')").bind(&c).bind(&ac.id).bind(vec![17u8;80]).bind(&a).execute(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let v = PgConnection::new(p.clone())
        .status(&a, &c, &ac.id)
        .await
        .unwrap();
    assert!(v.configured && v.schedule_enabled);
    assert!(!v.matching_enabled);
    assert!(!v.matching_treasury);
    assert!(v.matching_next_at.is_none());
    assert_eq!(v.revision, 4);
    assert!(!PgBank(p).process_matching_one().await.unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn matching_rest_mcp_configuration_and_status(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, token, c, _) = setup(&p).await;
    let ac = account(&PgBank(p.clone()), &a, &c).await;
    let app =
        faktura_server::banking::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let command = serde_json::json!({"account_id":ac.id,"revision":0,"request_id":key(),"action":"matching","enabled":true,"include_treasury":true});
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/bank/connection"))
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
    let rest: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(rest["matching_enabled"], true);
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
    let args = serde_json::json!({"company_id":c,"account_id":ac.id});
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_connection_status")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let json = serde_json::to_value(r).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rest, mcp);
    let args = serde_json::json!({"company_id":c,"account_id":ac.id,"revision":1,"request_id":key(),"action":"matching","enabled":false});
    let r = client
        .call_tool(
            CallToolRequestParams::new("bank_connection_execute")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    assert!(
        !PgConnection::new(p)
            .status(&a, &c, &ac.id)
            .await
            .unwrap()
            .matching_enabled
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

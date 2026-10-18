use super::*;
use super::{
    automation::{due, enable},
    batch_tests::for_invoice,
    foreign_tests::foreign_invoice,
    native_matching::eur,
};
use faktura_application::bank_connection::ConnectionStore;
use faktura_infrastructure::bank_connection::PgConnection;
fn input(ids: Vec<String>) -> banking::BatchInput {
    banking::BatchInput { movement_ids: ids }
}
fn confirm(ids: Vec<String>, fingerprint: String) -> Command {
    Command {
        request_id: key(),
        action: Action::BatchMatch {
            movement_ids: ids,
            fingerprint,
        },
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_batch_mixed_currencies_do_not_compete_and_replay(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let euro = eur(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let Reply::Account(czk) = run(
        &s,
        &a,
        &c,
        Action::Account {
            id: None,
            revision: None,
            archived: false,
            data: AccountData {
                name: "CZK".into(),
                iban: "CZ3808000000192000145400".into(),
                currency: "CZK".into(),
            },
        },
    )
    .await
    else {
        panic!()
    };
    let mut data = d.data.clone();
    data.calculation.currency = "CZK".into();
    data.calculation.exchange_rate = None;
    let store = PgDraft(p.clone());
    let draft = draft::create(
        &store,
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data,
        },
    )
    .await
    .unwrap();
    let local = draft::issue(
        &store,
        &a,
        &c,
        &draft.id,
        draft::Issue {
            request_id: key(),
            revision: draft.revision,
        },
    )
    .await
    .unwrap();
    let m1 = for_invoice(&s, &a, &c, &euro, &d, "100").await;
    let m2 = for_invoice(&s, &a, &c, &czk, &local, "100").await;
    let ids = vec![m1.id.clone(), m2.id.clone()];
    let preview = s.batch_preview(&a, &c, input(ids.clone())).await.unwrap();
    assert_eq!(preview.ready_count, 2);
    for item in &preview.items {
        assert_eq!(item.status, "ready");
        assert_eq!(
            item.candidate.as_ref().unwrap().document_id,
            if item.movement.data.currency == "EUR" {
                &d.id
            } else {
                &local.id
            }
            .clone()
        );
    }
    let cmd = confirm(ids, preview.fingerprint);
    let repeat = serde_json::to_value(&cmd).unwrap();
    let one = s.execute(&a, &c, cmd).await.unwrap();
    let two = s
        .execute(&a, &c, serde_json::from_value(repeat).unwrap())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(one).unwrap(),
        serde_json::to_value(two).unwrap()
    );
    for id in [&d.id, &local.id] {
        let paid = PgPayment(p.clone()).get(&a, &c, id).await.unwrap();
        assert_eq!(paid.items.len(), 1);
        assert_eq!(paid.summary.outstanding, "0.00");
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_batch_same_currency_competitor_and_stale_preview(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = eur(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let m = for_invoice(&s, &a, &c, &ac, &d, "100").await;
    let report = s
        .batch_preview(&a, &c, input(vec![m.id.clone()]))
        .await
        .unwrap();
    assert_eq!(report.ready_count, 1);
    let Reply::Account(other) = run(
        &s,
        &a,
        &c,
        Action::Account {
            id: None,
            revision: None,
            archived: false,
            data: AccountData {
                name: "Other EUR".into(),
                iban: "CZ3808000000192000145400".into(),
                currency: "EUR".into(),
            },
        },
    )
    .await
    else {
        panic!()
    };
    for_invoice(&s, &a, &c, &other, &d, "100").await;
    let next = s
        .batch_preview(&a, &c, input(vec![m.id.clone()]))
        .await
        .unwrap();
    assert_eq!(next.ready_count, 0);
    assert_eq!(next.items[0].status, "competing_movement");
    assert!(s
        .execute(&a, &c, confirm(vec![m.id], report.fingerprint))
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
async fn native_worker_existing_payment_concurrency_detachment_and_permission(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = eur(&s, &a, &c).await;
    let d = foreign_invoice(&p, &a, &c).await;
    let paid = payment::register(
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
                note: "Native worker".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = for_invoice(&s, &a, &c, &ac, &d, "100").await;
    assert!(!s.process_matching_one().await.unwrap());
    enable(&p, &a, &c, &ac).await;
    let (one, two) = tokio::join!(s.process_matching_one(), s.process_matching_one());
    one.unwrap();
    two.unwrap();
    let linked = s.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(linked.matches.len(), 1);
    assert!(!linked.matches[0].created_payment);
    assert_eq!(
        serde_json::to_value(&paid).unwrap(),
        serde_json::to_value(PgPayment(p.clone()).get(&a, &c, &d.id).await.unwrap()).unwrap()
    );
    unlink(&s, &a, &c, &linked).await;
    due(&p).await;
    s.process_matching_one().await.unwrap();
    let linked = s.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(linked.matches.len(), 1);
    assert!(linked.matches[0].detached_at.is_some());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    due(&p).await;
    s.process_matching_one().await.unwrap();
    let status = PgConnection::new(p.clone())
        .status(&a, &c, &ac.id)
        .await
        .unwrap();
    assert!(!status.matching_enabled);
    assert_eq!(
        status.matching_error.as_deref(),
        Some("matching_permission")
    );
}

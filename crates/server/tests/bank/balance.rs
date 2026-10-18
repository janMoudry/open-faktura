use super::*;
use faktura_application::banking::BalanceQuery;
fn q(ac: &Account) -> BalanceQuery {
    BalanceQuery {
        account_id: ac.id.clone(),
        ..Default::default()
    }
}
fn opening(ac: &Account) -> Command {
    Command {
        request_id: key(),
        action: Action::Opening {
            account_id: ac.id.clone(),
            revision: ac.revision,
            opening_on: "2026-10-01".parse().unwrap(),
            opening_balance: "1000.00".into(),
            reason: "Synthetic opening statement".into(),
        },
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn period_balance_all_pages_replay_revision_scope(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    assert!(s.balance(&a, &c, q(&ac)).await.unwrap().closing.is_none());
    for _ in 0..26 {
        movement(&s, &a, &c, &ac, "100.01").await;
    }
    movement(&s, &a, &c, &ac, "-50.02").await;
    let cmd = opening(&ac);
    let retry = serde_json::to_value(&cmd).unwrap();
    let Reply::Account(new) = s.execute(&a, &c, cmd).await.unwrap() else {
        panic!()
    };
    s.execute(&a, &c, serde_json::from_value(retry).unwrap())
        .await
        .unwrap();
    assert!(s.execute(&a, &c, opening(&ac)).await.is_err());
    let v = s.balance(&a, &c, q(&ac)).await.unwrap();
    assert_eq!(v.count, 27);
    assert_eq!(v.income, "2600.26");
    assert_eq!(v.expense, "50.02");
    assert_eq!(v.closing.as_deref(), Some("3550.24"));
    let mut period = q(&ac);
    period.from = Some("2026-10-02".parse().unwrap());
    let v = s.balance(&a, &c, period).await.unwrap();
    assert_eq!(v.opening.as_deref(), Some("3550.24"));
    assert_eq!(v.income, "0.00");
    let mut invalid = q(&ac);
    invalid.from = Some("2026-09-30".parse().unwrap());
    assert!(s.balance(&a, &c, invalid).await.is_err());
    assert!(s.balance(&a, &other, q(&ac)).await.is_err());
    assert!(s.execute(&a, &other, opening(&new)).await.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.balance(&a, &c, q(&ac)).await.is_ok());
    assert!(s.execute(&a, &c, opening(&new)).await.is_err());
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade031_does_not_invent_opening_balance(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140032)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    movement(&s, &a, &c, &ac, "100").await;
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    // Deployment restarts the API; discard pre-migration prepared SELECT * plans.
    let fresh = PgPool::connect_with((*p.connect_options()).clone())
        .await
        .unwrap();
    let s = PgBank(fresh.clone());
    let v = s.balance(&a, &c, q(&ac)).await.unwrap();
    fresh.close().await;
    assert!(v.opening.is_none());
    assert_eq!(v.income, "100.00");
    assert!(v.closing.is_none());
}

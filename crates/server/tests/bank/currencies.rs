use super::*;
use faktura_application::banking::{BalanceQuery, FileInput};
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_currency_import_balance_guards_and_scope(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
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
                name: "EUR account".into(),
                iban: "CZ6508000000192000145399".into(),
                currency: "EUR".into(),
            },
        },
    )
    .await
    else {
        panic!()
    };
    let csv = "external_id,booked_on,amount,currency,variable_symbol,counterparty,description\ne1,2026-10-01,100.01,EUR,123,Customer,\ne2,2026-10-02,-25.02,EUR,,Vendor,\n";
    let file = |csv: String| FileInput {
        account_id: ac.id.clone(),
        csv,
    };
    assert!(s.preview(&a, &c, file(csv.into())).await.unwrap().valid);
    let bad = csv.replace("-25.02,EUR", "-25.02,CZK");
    assert!(!s.preview(&a, &c, file(bad.clone())).await.unwrap().valid);
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
        s.balance(
            &a,
            &c,
            BalanceQuery {
                account_id: ac.id.clone(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .count,
        0
    );
    for _ in 0..2 {
        run(
            &s,
            &a,
            &c,
            Action::Import {
                account_id: ac.id.clone(),
                csv: csv.into(),
            },
        )
        .await;
    }
    run(
        &s,
        &a,
        &c,
        Action::Opening {
            account_id: ac.id.clone(),
            revision: ac.revision,
            opening_on: "2026-10-01".parse().unwrap(),
            opening_balance: "50.00".into(),
            reason: "Synthetic opening".into(),
        },
    )
    .await;
    let balance = s
        .balance(
            &a,
            &c,
            BalanceQuery {
                account_id: ac.id.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(balance.currency, "EUR");
    assert_eq!(balance.count, 2);
    assert_eq!(balance.income, "100.01");
    assert_eq!(balance.expense, "25.02");
    assert_eq!(balance.closing.as_deref(), Some("124.99"));
    assert!(s.preview(&a, &other, file(csv.into())).await.is_err());
    let mid: i64 = sqlx::query_scalar("SELECT id FROM bank_movement WHERE external_id='e1'")
        .fetch_one(&p)
        .await
        .unwrap();
    assert!(s
        .candidates(&a, &c, &mid.to_string(), "")
        .await
        .unwrap()
        .is_empty());
    assert!(s
        .suggestions(&a, &c, &mid.to_string())
        .await
        .unwrap()
        .is_empty());
    let invoice = issued_case(&p, &a, &c, 30, false).await;
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Match {
                    movement_id: mid.to_string(),
                    revision: 1,
                    target_kind: "invoice".into(),
                    document_id: invoice.id,
                    target_revision: 1,
                    payment_id: None
                }
            }
        )
        .await
        .is_err());
    let mut changed = ac.data.clone();
    changed.currency = "CZK".into();
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Account {
                    id: Some(ac.id.clone()),
                    revision: Some(2),
                    data: changed,
                    archived: false
                }
            }
        )
        .await
        .is_err());
    // Database also rejects a bypass of the command's currency validation.
    assert!(sqlx::query(
        "UPDATE bank_movement SET data=jsonb_set(data,'{currency}','\"CZK\"') WHERE id=$1"
    )
    .bind(mid)
    .execute(&p)
    .await
    .is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Import {
                    account_id: ac.id,
                    csv: csv.into()
                }
            }
        )
        .await
        .is_err());
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade053_keeps_native_czk_history(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610170054)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgBank(p.clone());
    let ac = account(&s, &a, &c).await;
    let m = movement(&s, &a, &c, &ac, "123.45").await;
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let fresh = PgPool::connect_with((*p.connect_options()).clone())
        .await
        .unwrap();
    let row: (String, String) =
        sqlx::query_as("SELECT currency,amount::text FROM bank_movement WHERE id=$1::text::bigint")
            .bind(m.id)
            .fetch_one(&fresh)
            .await
            .unwrap();
    assert_eq!(row, ("CZK".into(), "123.45".into()));
    fresh.close().await;
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_iban_normalization_import_dedup_and_immutability(p: PgPool) {
    use base64::Engine;
    let (a, _, c, other) = setup(&p).await;
    let s = PgBank(p.clone());
    let create = || Action::Account {
        id: None,
        revision: None,
        archived: false,
        data: AccountData {
            name: "Foreign EUR".into(),
            iban: "de89 3704 0044 0532 0130 00".into(),
            currency: "EUR".into(),
        },
    };
    let Reply::Account(ac) = run(&s, &a, &c, create()).await else {
        panic!()
    };
    assert_eq!(ac.data.iban, "DE89370400440532013000");
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: create()
            }
        )
        .await
        .is_err());
    // Same identifier is allowed in another profile, but its movements remain isolated.
    run(&s, &a, &other, create()).await;
    let xml = include_str!("../../../../tests/fixtures/bank/camt053.xml")
        .replace("CZ6508000000192000145399", &ac.data.iban)
        .replace("CZK", "EUR");
    let file = |xml: &str| banking::StatementInput {
        account_id: ac.id.clone(),
        format: "camt053".into(),
        filename: "synthetic-de.xml".into(),
        content_base64: base64::engine::general_purpose::STANDARD.encode(xml),
    };
    assert!(s.statement_preview(&a, &c, file(&xml)).await.unwrap().valid);
    for invalid in [
        xml.replace(&ac.data.iban, "GB82WEST12345698765432"),
        xml.replace("EUR", "USD"),
    ] {
        assert!(s.statement_preview(&a, &c, file(&invalid)).await.is_err());
        assert!(s
            .execute(
                &a,
                &c,
                Command {
                    request_id: key(),
                    action: Action::Statement {
                        file: file(&invalid)
                    }
                }
            )
            .await
            .is_err());
    }
    assert!(s.statement_preview(&a, &other, file(&xml)).await.is_err());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        0
    );
    for _ in 0..2 {
        run(&s, &a, &c, Action::Statement { file: file(&xml) }).await;
    }
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        2
    );
    assert_eq!(
        s.movements(&a, &other, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
    let balance = s
        .balance(
            &a,
            &c,
            BalanceQuery {
                account_id: ac.id.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(balance.currency, "EUR");
    assert_eq!(balance.income, "100.01");
    assert_eq!(balance.expense, "25.00");
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Account {
                    id: Some(ac.id.clone()),
                    revision: Some(ac.revision),
                    archived: false,
                    data: AccountData {
                        iban: "GB82WEST12345698765432".into(),
                        ..ac.data.clone()
                    },
                }
            }
        )
        .await
        .is_err());
    assert_eq!(s.accounts(&a, &c).await.unwrap()[0].data.iban, ac.data.iban);
}

use super::*;
async fn enable(s: &PgCash, a: &str, c: &str, account: Option<&Account>) {
    let v = s.settings(a, c).await.unwrap();
    run(
        s,
        a,
        c,
        Action::Settings {
            account_id: account.map(|a| a.id.clone()),
            revision: v.revision,
        },
    )
    .await;
}
fn pay(d: &draft::Draft, revision: i64, value: &str) -> payment::Register {
    payment::Register {
        request_id: key(),
        revision,
        payment: PaymentInput {
            amount: value.into(),
            paid_on: d.data.issued_on.unwrap(),
            method: "cash".into(),
            note: "Synthetic cash".into(),
        },
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn automatic_invoice_cash_atomic_replay_replace_and_disable(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let cash = PgCash(p.clone());
    let ac = account(&cash, &a, &c).await;
    enable(&cash, &a, &c, Some(&ac)).await;
    let invoice = issued_case(&p, &a, &c, 0, false).await;
    let payments = PgPayment(p.clone());
    let cmd = pay(&invoice, 0, "100");
    let retry = serde_json::to_value(&cmd).unwrap();
    let result = payment::register(&payments, &a, &c, &invoice.id, cmd)
        .await
        .unwrap();
    payment::register(
        &payments,
        &a,
        &c,
        &invoice.id,
        serde_json::from_value(retry).unwrap(),
    )
    .await
    .unwrap();
    let list = cash.movements(&a, &c, Default::default()).await.unwrap();
    assert_eq!(list.total, 1);
    let m = &list.items[0];
    assert!(m.automatic);
    assert_eq!(m.data.amount, "100.00");
    assert_eq!(m.matches[0].payment_id, result.items[0].id);
    assert!(cash.get(&a, &other, &m.id).await.is_err());
    assert!(cash
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Unmatch {
                    movement_id: m.id.clone(),
                    revision: m.revision,
                    reason: "Not from source".into()
                }
            }
        )
        .await
        .is_err());
    let cmd = payment::Reverse {
        request_id: key(),
        revision: result.summary.revision,
        reason: "Correct amount".into(),
        replacement: Some(pay(&invoice, 0, "75").payment),
    };
    let result = payment::reverse(&payments, &a, &c, &invoice.id, &result.items[0].id, cmd)
        .await
        .unwrap();
    assert_eq!(result.summary.paid, "75.00");
    let list = cash.movements(&a, &c, Default::default()).await.unwrap();
    assert_eq!(list.total, 2);
    assert_eq!(list.items.iter().filter(|v| v.cancelled).count(), 1);
    let balance = cash
        .balance(
            &a,
            &c,
            cash::Search {
                account_id: ac.id.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(balance.closing, "1075.00");
    enable(&cash, &a, &c, None).await;
    let current = result
        .items
        .iter()
        .find(|p| p.reversed_at.is_none())
        .unwrap();
    payment::reverse(
        &payments,
        &a,
        &c,
        &invoice.id,
        &current.id,
        payment::Reverse {
            request_id: key(),
            revision: result.summary.revision,
            reason: "Refund".into(),
            replacement: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        cash.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .items
            .iter()
            .filter(|m| !m.cancelled)
            .count(),
        0
    );
    // No retrospective documents when enabling after a payment.
    let current = payments.get(&a, &c, &invoice.id).await.unwrap();
    let before = pay(&invoice, current.summary.revision, "10");
    let again = serde_json::to_value(&before).unwrap();
    payment::register(&payments, &a, &c, &invoice.id, before)
        .await
        .unwrap();
    enable(&cash, &a, &c, Some(&ac)).await;
    payment::register(
        &payments,
        &a,
        &c,
        &invoice.id,
        serde_json::from_value(again).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        cash.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        2
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn automatic_receipt_update_card_cancel_and_received_invoice(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let cash = PgCash(p.clone());
    let ac = account(&cash, &a, &c).await;
    enable(&cash, &a, &c, Some(&ac)).await;
    let store = PgReceived(p.clone());
    let mut input = data();
    input.kind = faktura_domain::received::Kind::Receipt;
    input.receipt_method = Some("cash".into());
    input.received_on = input.issued_on;
    input.due_on = input.issued_on;
    input.bank_account.clear();
    input.variable_symbol.clear();
    let doc = received::create(
        &store,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: input.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        cash.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
    input.lines[0].net = "200.01".into();
    let doc = received::command(
        &store,
        &a,
        &c,
        &doc.id,
        received::Command {
            request_id: key(),
            revision: doc.revision,
            action: received::Action::Update {
                data: Box::new(input.clone()),
            },
        },
    )
    .await
    .unwrap();
    let list = cash.movements(&a, &c, Default::default()).await.unwrap();
    assert_eq!(list.total, 2);
    assert_eq!(list.items.iter().filter(|m| !m.cancelled).count(), 1);
    assert_eq!(list.items[0].data.amount, "-221.00");
    input.receipt_method = Some("card".into());
    let doc = received::command(
        &store,
        &a,
        &c,
        &doc.id,
        received::Command {
            request_id: key(),
            revision: doc.revision,
            action: received::Action::Update {
                data: Box::new(input),
            },
        },
    )
    .await
    .unwrap();
    assert!(cash
        .movements(&a, &c, Default::default())
        .await
        .unwrap()
        .items
        .iter()
        .all(|m| m.cancelled));
    received::command(
        &store,
        &a,
        &c,
        &doc.id,
        received::Command {
            request_id: key(),
            revision: doc.revision,
            action: received::Action::Cancel {
                reason: "Void card receipt".into(),
            },
        },
    )
    .await
    .unwrap();
    let doc = received::create(
        &store,
        &a,
        &c,
        received::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let doc = received::command(
        &store,
        &a,
        &c,
        &doc.id,
        received::Command {
            request_id: key(),
            revision: doc.revision,
            action: received::Action::Pay {
                payment: PaymentInput {
                    amount: "50".into(),
                    paid_on: doc.data.issued_on,
                    method: "cash".into(),
                    note: "".into(),
                },
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(
        cash.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .items[0]
            .data
            .amount,
        "-50.00"
    );
    received::command(
        &store,
        &a,
        &c,
        &doc.id,
        received::Command {
            request_id: key(),
            revision: doc.revision,
            action: received::Action::Reverse {
                payment_id: doc.payments[0].id.clone(),
                reason: "Refund".into(),
            },
        },
    )
    .await
    .unwrap();
    assert!(cash
        .movements(&a, &c, Default::default())
        .await
        .unwrap()
        .items
        .iter()
        .all(|m| m.cancelled));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn settings_scope_date_rollback_manual_matching_no_duplicate(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let cash = PgCash(p.clone());
    let ac = account(&cash, &a, &c).await;
    enable(&cash, &a, &c, Some(&ac)).await;
    assert!(cash
        .execute(
            &a,
            &other,
            Command {
                request_id: key(),
                action: Action::Settings {
                    account_id: Some(ac.id.clone()),
                    revision: 0
                }
            }
        )
        .await
        .is_err());
    assert!(cash
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Account {
                    id: Some(ac.id.clone()),
                    revision: Some(1),
                    data: ac.data.clone(),
                    archived: true
                }
            }
        )
        .await
        .is_err());
    let d = issued_case(&p, &a, &c, 20, false).await;
    let m = movement(&cash, &a, &c, &ac, "100").await;
    let candidate = cash.candidates(&a, &c, &m.id, "").await.unwrap().remove(0);
    cash.execute(&a, &c, match_cmd(&m, &candidate))
        .await
        .unwrap();
    assert_eq!(
        cash.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
    // New cash date outside the configured register rejects the entire payment.
    let future = AccountData {
        name: "Later cash".into(),
        opening_on: "2026-10-13".parse().unwrap(),
        opening_balance: "0.00".into(),
        currency: "CZK".into(),
    };
    let Reply::Account(later) = run(
        &cash,
        &a,
        &c,
        Action::Account {
            id: None,
            revision: None,
            data: future,
            archived: false,
        },
    )
    .await
    else {
        panic!()
    };
    enable(&cash, &a, &c, Some(&later)).await;
    let payments = PgPayment(p.clone());
    let current = payments.get(&a, &c, &d.id).await.unwrap();
    let cmd = pay(&d, current.summary.revision, "25");
    let retry = serde_json::to_value(&cmd).unwrap();
    assert!(payment::register(&payments, &a, &c, &d.id, cmd)
        .await
        .is_err());
    assert_eq!(
        payments.get(&a, &c, &d.id).await.unwrap().summary.paid,
        "100.00"
    );
    enable(&cash, &a, &c, Some(&ac)).await;
    payment::register(
        &payments,
        &a,
        &c,
        &d.id,
        serde_json::from_value(retry).unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        cash.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        2
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    let settings = cash.settings(&a, &c).await.unwrap();
    assert!(cash
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Settings {
                    account_id: None,
                    revision: settings.revision
                }
            }
        )
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn cash_pdf_snapshot_cache_cs_en_scope_and_cancel(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let cash = PgCash(p.clone());
    let ac = account(&cash, &a, &c).await;
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{name}',to_jsonb('Žluťoučký kůň s.r.o.'::text)) WHERE id=$1::text::bigint").bind(&c).execute(&p).await.unwrap();
    let m = movement(&cash, &a, &c, &ac, "1234.56").await;
    let cs = cash.pdf(&a, &c, &m.id, "cs").await.unwrap();
    assert!(cs.starts_with(b"%PDF"));
    assert!(cash.pdf(&a, &other, &m.id, "cs").await.is_err());
    assert!(cash.pdf(&a, &c, &m.id, "de").await.is_err());
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{name}',to_jsonb('Changed profile'::text)) WHERE id=$1::text::bigint").bind(&c).execute(&p).await.unwrap();
    assert_eq!(cs, cash.pdf(&a, &c, &m.id, "cs").await.unwrap());
    let en = cash.pdf(&a, &c, &m.id, "en").await.unwrap();
    assert!(en.starts_with(b"%PDF"));
    if let Ok(dir) = std::env::var("CASH_PDF_TEST_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/cash-cs.pdf"), &cs).unwrap();
        std::fs::write(format!("{dir}/cash-en.pdf"), &en).unwrap();
    }
    run(
        &cash,
        &a,
        &c,
        Action::Cancel {
            movement_id: m.id.clone(),
            revision: m.revision,
            reason: "Synthetic cancellation".into(),
        },
    )
    .await;
    let cancelled = cash.pdf(&a, &c, &m.id, "cs").await.unwrap();
    assert_ne!(cancelled, cs);
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(cash.pdf(&a, &c, &m.id, "cs").await.is_ok());
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_029_snapshots_existing_vouchers_without_enabling_automation(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140030)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let account:i64=sqlx::query_scalar("INSERT INTO cash_account(company_id,name,currency,opening_on,opening_balance) VALUES($1::text::bigint,'Legacy','CZK','2026-01-01',1000) RETURNING id").bind(&c).fetch_one(&p).await.unwrap();
    let data = MovementData {
        booked_on: "2026-10-01".parse().unwrap(),
        amount: "100.00".into(),
        currency: "CZK".into(),
        variable_symbol: "".into(),
        counterparty: "Legacy counterparty".into(),
        person: "".into(),
        description: "Legacy voucher".into(),
    };
    let id:i64=sqlx::query_scalar("INSERT INTO cash_movement(company_id,account_id,number,data,amount,booked_on) VALUES($1::text::bigint,$2,'PP2026000001',$3,100,'2026-10-01') RETURNING id").bind(&c).bind(account).bind(serde_json::to_value(data).unwrap()).fetch_one(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let s = PgCash(p.clone());
    assert!(s.settings(&a, &c).await.unwrap().account_id.is_none());
    assert!(!s.get(&a, &c, &id.to_string()).await.unwrap().automatic);
    assert_eq!(
        s.balance(
            &a,
            &c,
            cash::Search {
                account_id: account.to_string(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .closing,
        "1100.00"
    );
    assert!(s
        .pdf(&a, &c, &id.to_string(), "cs")
        .await
        .unwrap()
        .starts_with(b"%PDF"));
}

use super::*;
use faktura_application::{
    advance::{self, AdvanceStore},
    credit_note::{self, CreditStore},
    sales,
};
use faktura_infrastructure::{advance::PgAdvance, credit_note::PgCredit, sales::PgSales};
fn result(r: Reply) -> Movement {
    if let Reply::Movement(m) = r {
        m
    } else {
        panic!()
    }
}
async fn advance_case(p: &PgPool, a: &str, c: &str, vat: bool) -> sales::Document {
    let mut invoice: DraftData = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/invoice-draft.json"
    ))
    .unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.calculation.tax_mode = if vat {
        TaxMode::VatPayer
    } else {
        TaxMode::NonPayer
    };
    invoice.supplier.vat_number = "CZ12345678".into();
    invoice.calculation.lines[0].unit_price = "1000".into();
    invoice.calculation.currency = "EUR".into();
    invoice.calculation.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "25".into(),
        units: "1".into(),
    });
    invoice.tax_on = None;
    invoice.calculation.lines[0].vat_percent = if vat { "21" } else { "0" }.into();
    invoice.issued_on = Some("2026-10-01".parse().unwrap());
    invoice.due_on = Some("2026-10-15".parse().unwrap());
    sales::create(
        &PgSales(p.clone()),
        a,
        c,
        sales::Create {
            request_id: key(),
            kind: sales::Kind::Advance,
            data: sales::Data {
                name: "Synthetic advance".into(),
                maturity_days: 14,
                status: "created".into(),
                invoice,
            },
        },
    )
    .await
    .unwrap()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_treasury_advance_bank_replay_scope_and_reverse(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let d = advance_case(&p, &a, &c, false).await;
    let m = dated_movement(&bank, &a, &c, &ac, "100", d.data.invoice.issued_on.unwrap()).await;
    let candidate = bank
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.target_kind == "advance")
        .unwrap();
    assert_eq!(candidate.document_id, d.id);
    let cmd = match_cmd(&m, &candidate);
    let replay = serde_json::to_value(&cmd).unwrap();
    assert!(bank
        .execute(&a, &other, serde_json::from_value(replay.clone()).unwrap())
        .await
        .is_err());
    let m = result(bank.execute(&a, &c, cmd).await.unwrap());
    assert_eq!(m.matches[0].target_kind, "advance");
    assert_eq!(m.remaining, "0.00");
    assert_eq!(
        result(
            bank.execute(&a, &c, serde_json::from_value(replay).unwrap())
                .await
                .unwrap()
        )
        .revision,
        m.revision
    );
    let store = PgAdvance(p.clone());
    let payments = store.payments(&a, &c, &d.id).await.unwrap();
    assert_eq!(payments.len(), 1);
    assert!(advance::reverse(
        &store,
        &a,
        &c,
        &d.id,
        &payments[0].id,
        advance::Reverse {
            revision: sales::get(&PgSales(p.clone()), &a, &c, &d.id)
                .await
                .unwrap()
                .revision,
            reason: "Direct reversal forbidden".into()
        }
    )
    .await
    .is_err());
    let m = unlink(&bank, &a, &c, &m).await;
    assert!(m.matches[0].detached_at.is_some());
    assert!(store.payments(&a, &c, &d.id).await.unwrap()[0].reversed);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_treasury_credit_refund_bank(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let invoice = super::foreign_tests::foreign_invoice(&p, &a, &c).await;
    let s = PgCredit(p.clone());
    let mut d = credit_note::create(
        &s,
        &a,
        &c,
        credit_note::Create {
            request_id: key(),
            invoice_id: invoice.id.clone(),
        },
    )
    .await
    .unwrap();
    d.data.reason = "Synthetic return".into();
    d = credit_note::command(
        &s,
        &a,
        &c,
        &d.id,
        credit_note::Command {
            request_id: key(),
            revision: d.revision,
            action: credit_note::Action::Update {
                data: d.data.clone(),
            },
        },
    )
    .await
    .unwrap();
    d = credit_note::command(
        &s,
        &a,
        &c,
        &d.id,
        credit_note::Command {
            request_id: key(),
            revision: d.revision,
            action: credit_note::Action::Issue,
        },
    )
    .await
    .unwrap();
    let m = dated_movement(&bank, &a, &c, &ac, "-100", d.data.issued_on).await;
    let candidate = bank
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.target_kind == "credit")
        .unwrap();
    let m = result(
        bank.execute(&a, &c, match_cmd(&m, &candidate))
            .await
            .unwrap(),
    );
    assert_eq!(m.matches[0].document_kind, "credit");
    let d = s.get(&a, &c, &d.id).await.unwrap();
    assert_eq!(d.refunded, "100.00");
    assert!(credit_note::command(
        &s,
        &a,
        &c,
        &d.id,
        credit_note::Command {
            request_id: key(),
            revision: d.revision,
            action: credit_note::Action::Reverse {
                event_id: m.matches[0].payment_id.clone(),
                reason: "Cannot bypass bank".into()
            }
        }
    )
    .await
    .is_err());
    unlink(&bank, &a, &c, &m).await;
    assert_eq!(s.get(&a, &c, &d.id).await.unwrap().refunded, "0.00");
    let d = s.get(&a, &c, &d.id).await.unwrap();
    let d = credit_note::command(
        &s,
        &a,
        &c,
        &d.id,
        credit_note::Command {
            request_id: key(),
            revision: d.revision,
            action: credit_note::Action::Refund {
                amount: "100".into(),
                date: d.data.issued_on,
                method: "bank_transfer".into(),
                reason: "Existing EUR refund".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = dated_movement(&bank, &a, &c, &ac, "-100", d.data.issued_on).await;
    let candidate = bank
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.target_kind == "credit" && v.payment_id.is_some())
        .unwrap();
    let linked = result(
        bank.execute(&a, &c, match_cmd(&m, &candidate))
            .await
            .unwrap(),
    );
    assert!(!linked.matches[0].created_payment);
    assert_eq!(s.get(&a, &c, &d.id).await.unwrap().refunded, "100.00");
    unlink(&bank, &a, &c, &linked).await;
    assert_eq!(s.get(&a, &c, &d.id).await.unwrap().refunded, "100.00");
}

async fn dated_movement(
    s: &PgBank,
    a: &str,
    c: &str,
    ac: &Account,
    value: &str,
    on: sqlx::types::chrono::NaiveDate,
) -> Movement {
    result(
        run(
            s,
            a,
            c,
            Action::Manual {
                account_id: ac.id.clone(),
                data: MovementData {
                    external_id: "".into(),
                    booked_on: on,
                    amount: value.into(),
                    currency: ac.data.currency.clone(),
                    variable_symbol: "".into(),
                    counterparty: "Synthetic".into(),
                    description: "Treasury".into(),
                },
            },
        )
        .await,
    )
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_treasury_tax_receipt_blocks_detachment_atomically(p: PgPool) {
    use faktura_application::tax_receipt;
    use faktura_infrastructure::tax_receipt::PgTaxReceipt;
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let d = advance_case(&p, &a, &c, true).await;
    let on = d.data.invoice.issued_on.unwrap();
    let invoice = super::foreign_tests::foreign_invoice(&p, &a, &c).await;
    let invoice_revision = PgPayment(p.clone())
        .get(&a, &c, &invoice.id)
        .await
        .unwrap()
        .summary
        .revision;
    let m = dated_movement(&bank, &a, &c, &ac, "221", on).await;
    let candidate = bank
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.target_kind == "advance")
        .unwrap();
    let m = result(
        bank.execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Split {
                    movement_id: m.id.clone(),
                    revision: m.revision,
                    allocations: vec![
                        banking::Allocation {
                            target_kind: "invoice".into(),
                            document_id: invoice.id.clone(),
                            target_revision: invoice_revision,
                            payment_id: None,
                            amount: "100".into(),
                        },
                        banking::Allocation {
                            target_kind: "advance".into(),
                            document_id: d.id.clone(),
                            target_revision: candidate.revision,
                            payment_id: None,
                            amount: "121".into(),
                        },
                    ],
                },
            },
        )
        .await
        .unwrap(),
    );
    let d = sales::get(&PgSales(p.clone()), &a, &c, &d.id)
        .await
        .unwrap();
    tax_receipt::issue(
        &PgTaxReceipt(p.clone()),
        &a,
        &c,
        tax_receipt::Issue {
            request_id: key(),
            revision: d.revision,
            advance_id: d.id.clone(),
            payment_id: m.matches[1].payment_id.clone(),
            issued_on: on,
            description: "Synthetic receipt".into(),
            language: "cs".into(),
        },
    )
    .await
    .unwrap();
    assert!(bank
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Unmatch {
                    movement_id: m.id.clone(),
                    revision: m.revision,
                    reason: "Must retain DDPP link".into()
                }
            }
        )
        .await
        .is_err());
    assert!(bank
        .get(&a, &c, &m.id)
        .await
        .unwrap()
        .matches
        .iter()
        .all(|v| v.detached_at.is_none()));
    assert_eq!(
        PgPayment(p.clone())
            .get(&a, &c, &invoice.id)
            .await
            .unwrap()
            .summary
            .paid,
        "100.00"
    );
    assert!(!PgAdvance(p.clone()).payments(&a, &c, &d.id).await.unwrap()[0].reversed);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_treasury_existing_advance_and_wrong_currency(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let d = advance_case(&p, &a, &c, false).await;
    let on = d.data.invoice.issued_on.unwrap();
    let store = PgAdvance(p.clone());
    advance::register(
        &store,
        &a,
        &c,
        &d.id,
        advance::Register {
            request_id: key(),
            revision: d.revision,
            payment: PaymentInput {
                amount: "100".into(),
                paid_on: on,
                method: "bank_transfer".into(),
                note: "Existing EUR payment".into(),
            },
        },
    )
    .await
    .unwrap();
    let m = dated_movement(&bank, &a, &c, &ac, "100", on).await;
    let candidate = bank
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .into_iter()
        .find(|v| v.target_kind == "advance" && v.payment_id.is_some())
        .unwrap();
    let linked = result(
        bank.execute(&a, &c, match_cmd(&m, &candidate))
            .await
            .unwrap(),
    );
    assert!(!linked.matches[0].created_payment);
    let second = dated_movement(&bank, &a, &c, &ac, "100", on).await;
    assert!(bank
        .execute(&a, &c, match_cmd(&second, &candidate))
        .await
        .is_err());
    unlink(&bank, &a, &c, &linked).await;
    let payments = store.payments(&a, &c, &d.id).await.unwrap();
    assert_eq!(payments.len(), 1);
    assert!(!payments[0].reversed);
    let Reply::Account(czk) = run(
        &bank,
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
    let m = dated_movement(&bank, &a, &c, &czk, "100", on).await;
    assert!(!bank
        .candidates(&a, &c, &m.id, "")
        .await
        .unwrap()
        .iter()
        .any(|v| v.target_kind == "advance" && v.document_id == d.id));
    assert!(bank
        .execute(&a, &c, match_cmd(&m, &candidate))
        .await
        .is_err());
    assert!(bank.get(&a, &c, &m.id).await.unwrap().matches.is_empty());
}

async fn enable_treasury(p: &PgPool, a: &str, c: &str, ac: &Account) {
    use faktura_application::bank_connection::{self as bc, ConnectionStore};
    let store = faktura_infrastructure::bank_connection::PgConnection::new(p.clone());
    let status = store.status(a, c, &ac.id).await.unwrap();
    let status = store
        .execute(
            a,
            c,
            bc::Command {
                account_id: ac.id.clone(),
                revision: status.revision,
                request_id: key(),
                action: bc::Action::Matching {
                    enabled: true,
                    include_treasury: true,
                },
            },
        )
        .await
        .unwrap();
    assert!(status.matching_treasury);
    super::automation::due(p).await;
}
#[allow(clippy::too_many_arguments)]
async fn reference_movement(
    _p: &PgPool,
    bank: &PgBank,
    a: &str,
    c: &str,
    ac: &Account,
    value: &str,
    symbol: &str,
) -> Movement {
    result(
        run(
            bank,
            a,
            c,
            Action::Manual {
                account_id: ac.id.clone(),
                data: MovementData {
                    external_id: "".into(),
                    booked_on: "2026-10-18".parse().unwrap(),
                    amount: value.into(),
                    currency: ac.data.currency.clone(),
                    variable_symbol: symbol.into(),
                    counterparty: "Synthetic".into(),
                    description: "Treasury automatic matching".into(),
                },
            },
        )
        .await,
    )
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn treasury_worker_explicit_opt_in_and_existing_payment(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let d = advance_case(&p, &a, &c, false).await;
    let symbol = &d.data.invoice.payment.variable_symbol;
    assert!(!symbol.is_empty());
    let m = reference_movement(
        &p,
        &bank,
        &a,
        &c,
        &ac,
        &d.totals.as_ref().unwrap().payable,
        symbol,
    )
    .await;
    super::automation::enable(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    assert!(bank.get(&a, &c, &m.id).await.unwrap().matches.is_empty());
    // An already recorded transfer must be linked, not inserted again.
    advance::register(
        &PgAdvance(p.clone()),
        &a,
        &c,
        &d.id,
        advance::Register {
            request_id: key(),
            revision: d.revision,
            payment: faktura_domain::payment::PaymentInput {
                amount: d.totals.as_ref().unwrap().payable.clone(),
                paid_on: "2026-10-18".parse().unwrap(),
                method: "bank_transfer".into(),
                note: "Existing transfer".into(),
            },
        },
    )
    .await
    .unwrap();
    enable_treasury(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    let linked = bank.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(linked.matches.len(), 1);
    assert!(!linked.matches[0].created_payment);
    assert_eq!(
        PgAdvance(p.clone())
            .payments(&a, &c, &d.id)
            .await
            .unwrap()
            .len(),
        1
    );
    unlink(&bank, &a, &c, &linked).await;
    super::automation::due(&p).await;
    bank.process_matching_one().await.unwrap();
    assert!(bank.get(&a, &c, &m.id).await.unwrap().matches[0]
        .detached_at
        .is_some());
    assert!(!PgAdvance(p).payments(&a, &c, &d.id).await.unwrap()[0].reversed);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn treasury_worker_credit_refund_uses_source_reference(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let source = super::foreign_tests::foreign_invoice(&p, &a, &c).await;
    let store = PgCredit(p.clone());
    let d = credit_note::create(
        &store,
        &a,
        &c,
        credit_note::Create {
            request_id: key(),
            invoice_id: source.id.clone(),
        },
    )
    .await
    .unwrap();
    let mut data = d.data.clone();
    data.reason = "Synthetic return".into();
    let d = credit_note::command(
        &store,
        &a,
        &c,
        &d.id,
        credit_note::Command {
            request_id: key(),
            revision: d.revision,
            action: credit_note::Action::Update { data },
        },
    )
    .await
    .unwrap();
    let d = credit_note::command(
        &store,
        &a,
        &c,
        &d.id,
        credit_note::Command {
            request_id: key(),
            revision: d.revision,
            action: credit_note::Action::Issue,
        },
    )
    .await
    .unwrap();
    let m = reference_movement(
        &p,
        &bank,
        &a,
        &c,
        &ac,
        &format!("-{}", d.remaining),
        &source.data.payment.variable_symbol,
    )
    .await;
    enable_treasury(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    let linked = bank.get(&a, &c, &m.id).await.unwrap();
    assert_eq!(linked.matches.len(), 1);
    assert!(linked.matches[0].created_payment);
    assert_eq!(linked.matches[0].target_kind, "credit");
    let audit_kind: String = sqlx::query_scalar("SELECT items->0->>'target_kind' FROM bank_matching_run WHERE company_id=$1::text::bigint ORDER BY id DESC LIMIT 1").bind(&c).fetch_one(&p).await.unwrap();
    assert_eq!(audit_kind, "credit");
    assert_eq!(store.get(&a, &c, &d.id).await.unwrap().remaining, "0.00");
    unlink(&bank, &a, &c, &linked).await;
    assert_eq!(store.get(&a, &c, &d.id).await.unwrap().refunded, "0.00");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn treasury_worker_ambiguous_advances_are_not_paid(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    let one = advance_case(&p, &a, &c, false).await;
    let two = advance_case(&p, &a, &c, false).await;
    // A user's duplicate reference, not two different auto-generated numbers.
    let mut data = two.data.clone();
    data.invoice.payment.variable_symbol = one.data.invoice.payment.variable_symbol.clone();
    sales::update(
        &PgSales(p.clone()),
        &a,
        &c,
        &two.id,
        sales::Update {
            archived: false,
            revision: two.revision,
            data,
        },
    )
    .await
    .unwrap();
    let m = reference_movement(
        &p,
        &bank,
        &a,
        &c,
        &ac,
        &one.totals.as_ref().unwrap().payable,
        &one.data.invoice.payment.variable_symbol,
    )
    .await;
    enable_treasury(&p, &a, &c, &ac).await;
    bank.process_matching_one().await.unwrap();
    assert!(bank.get(&a, &c, &m.id).await.unwrap().matches.is_empty());
    assert!(PgAdvance(p.clone())
        .payments(&a, &c, &one.id)
        .await
        .unwrap()
        .is_empty());
    assert!(PgAdvance(p)
        .payments(&a, &c, &two.id)
        .await
        .unwrap()
        .is_empty());
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade056_keeps_treasury_matching_disabled(p: PgPool) {
    use faktura_application::bank_connection::ConnectionStore;
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610180057)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = super::native_matching::eur(&bank, &a, &c).await;
    sqlx::query("INSERT INTO bank_connection(company_id,account_id,matching_enabled,matching_actor_id,matching_next_at) VALUES($1::text::bigint,$2::text::bigint,true,$3::text::bigint,now())").bind(&c).bind(&ac.id).bind(&a).execute(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let status = faktura_infrastructure::bank_connection::PgConnection::new(p)
        .status(&a, &c, &ac.id)
        .await
        .unwrap();
    assert!(status.matching_enabled);
    assert!(!status.matching_treasury);
}

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

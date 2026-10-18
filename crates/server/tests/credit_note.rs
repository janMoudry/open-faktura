#[path = "support/historical_documents.rs"]
mod historical_documents;
use faktura_application::{auth, company, sales::Data};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> Data {
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.supplier.vat_number = "CZ12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::VatPayer;
    invoice.calculation.lines[0].vat_percent = "21".into();
    invoice.tax_on = None;
    invoice.calculation.lines[0].unit_price = "1000".into();
    invoice.calculation.lines[0].quantity = "1".into();
    Data {
        name: "Test quotation".into(),
        maturity_days: 14,
        status: "created".into(),
        invoice,
    }
}
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        pool.clone(),
        Some("synthetic-catalog-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-catalog-bootstrap-key-only-tests",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (actor, token) = auth::login(&auth, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let first = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        KEY,
        CompanySettings {
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..CompanySettings::default()
        },
    )
    .await
    .unwrap();
    let second = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        "00000000-0000-4000-8000-000000000002",
        CompanySettings {
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..CompanySettings::default()
        },
    )
    .await
    .unwrap();
    (actor.id, token, first.id, second.id)
}

use faktura_application::{
    credit_note::{self, Action, Command, Create, CreditStore},
    draft::{self, DraftStore},
    payment::{self, PaymentStore},
};
use faktura_infrastructure::{credit_note::PgCredit, draft::PgDraft, payment::PgPayment};
async fn invoice(pool: &PgPool, a: &str, c: &str) -> faktura_application::draft::Draft {
    let mut data = data().invoice;
    data.tax_on = data.issued_on;
    let v = draft::create(
        &PgDraft(pool.clone()),
        a,
        c,
        draft::Create {
            request_id: KEY.into(),
            data,
        },
    )
    .await
    .unwrap();
    draft::issue(
        &PgDraft(pool.clone()),
        a,
        c,
        &v.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: v.revision,
        },
    )
    .await
    .unwrap()
}
fn cmd(rev: i64, action: Action) -> Command {
    Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        revision: rev,
        action,
    }
}
async fn prepare(s: &PgCredit, a: &str, c: &str, i: &str) -> credit_note::Document {
    let mut v = credit_note::create(
        s,
        a,
        c,
        Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            invoice_id: i.into(),
        },
    )
    .await
    .unwrap();
    v.data.reason = "Synthetic return".into();
    credit_note::command(
        s,
        a,
        c,
        &v.id,
        cmd(
            v.revision,
            Action::Update {
                data: v.data.clone(),
            },
        ),
    )
    .await
    .unwrap()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn partial_credit_setoff_reversal_pdf_and_original_history(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let original = invoice(&pool, &a, &c).await;
    let s = PgCredit(pool.clone());
    let original_pdf = PgDraft(pool.clone())
        .pdf(&a, &c, &original.id)
        .await
        .unwrap();
    let mut v = prepare(&s, &a, &c, &original.id).await;
    v.data.lines[0].amount = "605".into();
    v = credit_note::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Update {
                data: v.data.clone(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(v.totals.amounts.net, "-500.00");
    assert_eq!(v.totals.amounts.vat, "-105.00");
    v = credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Issue))
        .await
        .unwrap();
    assert_eq!(v.number.as_deref(), Some("DB20260001"));
    let pdf = s.pdf(&a, &c, &v.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(s.get(&a, &other, &v.id).await.is_err());
    assert!(s.pdf(&a, &other, &v.id).await.is_err());
    v = credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Offset))
        .await
        .unwrap();
    assert_eq!(v.remaining, "0.00");
    let payments = PgPayment(pool.clone());
    let p = payments.get(&a, &c, &original.id).await.unwrap();
    assert_eq!(p.summary.outstanding, "605.00");
    assert!(p.items.is_empty());
    assert_eq!(payments.overview(&a, &c).await.unwrap().received, "0.00");
    assert!(
        credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Offset))
            .await
            .is_err()
    );
    v = credit_note::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Reverse {
                event_id: v.events[0].id.clone(),
                reason: "Synthetic correction".into(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(v.remaining, "605.00");
    assert_eq!(
        payments
            .get(&a, &c, &original.id)
            .await
            .unwrap()
            .summary
            .outstanding,
        "1210.00"
    );
    assert_eq!(s.pdf(&a, &c, &v.id).await.unwrap(), pdf);
    assert_eq!(
        PgDraft(pool.clone())
            .pdf(&a, &c, &original.id)
            .await
            .unwrap(),
        original_pdf
    );
    let mut second = prepare(&s, &a, &c, &original.id).await;
    assert_eq!(second.data.lines[0].amount, "605.00");
    second = credit_note::command(&s, &a, &c, &second.id, cmd(second.revision, Action::Issue))
        .await
        .unwrap();
    assert_eq!(second.totals.payable, "-605.00");
    assert!(credit_note::create(
        &s,
        &a,
        &c,
        Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            invoice_id: original.id
        }
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_issue_replay_roles_refunds_and_cancel(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let original = invoice(&pool, &a, &c).await;
    let s = PgCredit(pool.clone());
    let first = prepare(&s, &a, &c, &original.id).await;
    let second = prepare(&s, &a, &c, &original.id).await;
    let key = uuid::Uuid::new_v4().to_string();
    let issue = || Command {
        request_id: key.clone(),
        revision: first.revision,
        action: Action::Issue,
    };
    let (x, y) = tokio::join!(
        credit_note::command(&s, &a, &c, &first.id, issue()),
        credit_note::command(&s, &a, &c, &second.id, cmd(second.revision, Action::Issue))
    );
    assert_ne!(x.is_ok(), y.is_ok());
    let mut v = x.or(y).unwrap();
    if v.id == first.id {
        assert_eq!(
            credit_note::command(&s, &a, &c, &first.id, issue())
                .await
                .unwrap()
                .id,
            v.id
        );
    }
    let refund = || Action::Refund {
        amount: "500".into(),
        date: v.data.issued_on,
        method: "cash".into(),
        reason: "Returned to customer".into(),
    };
    v = credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, refund()))
        .await
        .unwrap();
    assert_eq!(v.refunded, "500.00");
    assert_eq!(v.remaining, "710.00");
    assert!(credit_note::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Refund {
                amount: "711".into(),
                date: v.data.issued_on,
                method: "cash".into(),
                reason: "Too much".into()
            }
        )
    )
    .await
    .is_err());
    v = credit_note::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Reverse {
                event_id: v.events[0].id.clone(),
                reason: "Correction".into(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(v.remaining, "1210.00");
    let loser = if v.id == first.id { second } else { first };
    credit_note::command(&s, &a, &c, &loser.id, cmd(loser.revision, Action::Cancel))
        .await
        .unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(s.get(&a, &c, &v.id).await.is_ok());
    assert!(
        credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Offset))
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn paid_final_invoice_keeps_advance_tax_history(pool: PgPool) {
    use advance::AdvanceStore;
    use faktura_application::{advance, advance_settlement, sales, tax_receipt};
    use faktura_infrastructure::{
        advance::PgAdvance, advance_settlement::PgSettlement, sales::PgSales,
        tax_receipt::PgTaxReceipt,
    };
    let (a, _, c, _) = setup(&pool).await;
    let z = sales::create(
        &PgSales(pool.clone()),
        &a,
        &c,
        sales::Create {
            request_id: KEY.into(),
            kind: sales::Kind::Advance,
            data: data(),
        },
    )
    .await
    .unwrap();
    advance::register(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &z.id,
        advance::Register {
            request_id: KEY.into(),
            revision: z.revision,
            payment: payment::PaymentInput {
                amount: "1210".into(),
                paid_on: "2026-10-11".parse().unwrap(),
                method: "cash".into(),
                note: String::new(),
            },
        },
    )
    .await
    .unwrap();
    let p = PgAdvance(pool.clone())
        .payments(&a, &c, &z.id)
        .await
        .unwrap()
        .remove(0);
    let receipt = tax_receipt::issue(
        &PgTaxReceipt(pool.clone()),
        &a,
        &c,
        tax_receipt::Issue {
            request_id: KEY.into(),
            revision: 2,
            advance_id: z.id.clone(),
            payment_id: p.id,
            issued_on: "2026-10-12".parse().unwrap(),
            description: "Advance".into(),
            language: "cs".into(),
        },
    )
    .await
    .unwrap();
    let d = advance_settlement::configure(
        &PgSettlement(pool.clone()),
        &a,
        &c,
        advance_settlement::Configure {
            request_id: KEY.into(),
            invoice_id: None,
            revision: None,
            advances: vec![advance_settlement::Selection {
                id: z.id.clone(),
                revision: 3,
            }],
        },
    )
    .await
    .unwrap();
    let issued = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &d.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(issued.totals.as_ref().unwrap().amounts.vat, "0.00");
    let s = PgCredit(pool.clone());
    let mut v = prepare(&s, &a, &c, &issued.id).await;
    assert_eq!(v.totals.amounts.vat, "-210.00");
    assert_eq!(v.totals.payable, "-1210.00");
    v = credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Issue))
        .await
        .unwrap();
    assert!(
        credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Offset))
            .await
            .is_err()
    );
    v = credit_note::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Refund {
                amount: "1210".into(),
                date: v.data.issued_on,
                method: "bank_transfer".into(),
                reason: "Refund advance".into(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(v.remaining, "0.00");
    let kept = draft::get(&PgDraft(pool.clone()), &a, &c, &issued.id)
        .await
        .unwrap()
        .advances
        .unwrap();
    assert_eq!(kept.sources[0].tax_receipts[0].id, receipt.id);
    assert_eq!(
        sales::get(&PgSales(pool.clone()), &a, &c, &z.id)
            .await
            .unwrap()
            .invoice_id,
        Some(issued.id)
    );
}
#[test]
fn cumulative_pennies_and_rounding_are_exact() {
    use faktura_domain::{
        credit_note::{calculate, Correction},
        invoice,
    };
    use std::collections::BTreeMap;
    let mut d = data().invoice;
    d.calculation.price_mode = invoice::PriceMode::Gross;
    d.calculation.lines[0].unit_price = "0.03".into();
    let totals = invoice::calculate(&d.calculation).unwrap();
    let id = d.calculation.lines[0].id.clone();
    let mut used = BTreeMap::new();
    let mut net = rust_decimal::Decimal::ZERO;
    let mut vat = net;
    for _ in 0..3 {
        let t = calculate(
            &d,
            &totals,
            &[Correction {
                return_quantity: None,
                id: id.clone(),
                amount: "0.01".into(),
            }],
            &used,
        )
        .unwrap();
        net += t.amounts.net.parse::<rust_decimal::Decimal>().unwrap();
        vat += t.amounts.vat.parse::<rust_decimal::Decimal>().unwrap();
        *used.entry(id.clone()).or_default() += rust_decimal::Decimal::new(1, 2);
    }
    assert_eq!(net, rust_decimal::Decimal::new(-2, 2));
    assert_eq!(vat, rust_decimal::Decimal::new(-1, 2));
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_and_input_contracts(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt;
    let (a, token, c, other) = setup(&pool).await;
    let original = invoice(&pool, &a, &c).await;
    let s = PgCredit(pool.clone());
    let v = prepare(&s, &a, &c, &original.id).await;
    let app = faktura_server::credit_note::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let command = cmd(v.revision, Action::Issue);
    let body = serde_json::to_vec(&command).unwrap();
    // Exercise serde flatten through the real boundary, not just typed calls.
    assert!(serde_json::from_slice::<Command>(&body).is_ok());
    let rejected = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/credit-notes/{}", v.id))
                .header("content-type", "application/json")
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), 401);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/credit-notes/{}", v.id))
                .header("content-type", "application/json")
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let issued: credit_note::Document =
        serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(issued.status, "issued");
    let denied = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{other}/credit-notes/{}/pdf", v.id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), 404);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token);
    let (ci, si) = tokio::io::duplex(65536);
    let task =
        tokio::spawn(async move { server.serve(si).await.unwrap().waiting().await.unwrap() });
    let client = ().serve(ci).await.unwrap();
    let mut args = serde_json::to_value(command)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    args.insert("company_id".into(), c.clone().into());
    args.insert("id".into(), v.id.clone().into());
    let result = client
        .call_tool(CallToolRequestParams::new("credit_command").with_arguments(args))
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    assert_eq!(s.list(&a, &c, Default::default()).await.unwrap().total, 1);
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade_020_keeps_invoice_pdf_and_cash(pool: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610120021)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let mut data = data().invoice;
    data.tax_on = data.issued_on;
    let (d, pdf) = historical_documents::invoice(
        &pool,
        &a,
        &c,
        draft::Create {
            request_id: KEY.into(),
            data,
        },
    )
    .await;
    payment::register(
        &PgPayment(pool.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: KEY.into(),
            revision: 0,
            payment: payment::PaymentInput {
                amount: "100".into(),
                paid_on: d.data.issued_on.unwrap(),
                method: "cash".into(),
                note: String::new(),
            },
        },
    )
    .await
    .unwrap();
    faktura_infrastructure::migrate(&pool).await.unwrap();
    assert_eq!(PgDraft(pool.clone()).pdf(&a, &c, &d.id).await.unwrap(), pdf);
    let p = PgPayment(pool.clone()).overview(&a, &c).await.unwrap();
    assert_eq!(p.received, "100.00");
    assert_eq!(p.outstanding, "1110.00");
    let s = PgCredit(pool.clone());
    let d = prepare(&s, &a, &c, &d.id).await;
    assert_eq!(d.totals.payable, "-1210.00");
}

#[test]
fn largest_supported_source_is_credited_without_decimal_overflow() {
    use faktura_domain::{
        credit_note::{calculate, Correction},
        invoice,
    };
    let mut d = data().invoice;
    d.calculation.lines[0].quantity = "1000000".into();
    d.calculation.lines[0].unit_price = "1000000000".into();
    let totals = invoice::calculate(&d.calculation).unwrap();
    let t = calculate(
        &d,
        &totals,
        &[Correction {
            return_quantity: None,
            id: d.calculation.lines[0].id.clone(),
            amount: "1210000000000000.00".into(),
        }],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(t.amounts.net, "-1000000000000000.00");
    assert_eq!(t.amounts.vat, "-210000000000000.00");
    assert_eq!(t.payable, "-1210000000000000.00");
}

#[test]
fn rounding_does_not_make_a_small_last_item_negative() {
    use faktura_domain::{
        credit_note::{calculate, capacities},
        invoice,
    };
    let mut d = data().invoice;
    d.calculation.tax_mode = invoice::TaxMode::NonPayer;
    d.calculation.rounding = invoice::Rounding::WholeCzk;
    d.calculation.lines[0].vat_percent = "0".into();
    d.calculation.lines[0].unit_price = "1.10".into();
    let mut small = d.calculation.lines[0].clone();
    small.id = "small".into();
    small.unit_price = "0.01".into();
    d.calculation.lines.push(small);
    let totals = invoice::calculate(&d.calculation).unwrap();
    assert_eq!(totals.rounding, "-0.11");
    let selections = capacities(&totals);
    assert_eq!(selections[1].amount, "0.01");
    let full = calculate(&d, &totals, &selections, &Default::default()).unwrap();
    assert_eq!(full.amounts.net, "-1.11");
    assert_eq!(full.rounding, "0.11");
    assert_eq!(full.payable, "-1.00");
    let first = calculate(&d, &totals, &selections[..1], &Default::default()).unwrap();
    let used = [(selections[0].id.clone(), "1.10".parse().unwrap())].into();
    let second = calculate(&d, &totals, &selections[1..], &used).unwrap();
    assert_eq!(first.payable, "-0.99");
    assert_eq!(second.payable, "-0.01");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_credit_rate_refund_offset_and_reports(pool: PgPool) {
    use faktura_application::{dashboard::DashboardStore, tax_report::TaxReportStore};
    let (a, _, c, other) = setup(&pool).await;
    let mut data = data().invoice;
    data.tax_on = data.issued_on;
    data.payment.method = "bank_transfer".into();
    data.payment.bank_account = "12345678/0100".into();
    data.calculation.currency = "EUR".into();
    data.calculation.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "25".into(),
        units: "1".into(),
    });
    data.calculation.lines[0].unit_price = "100".into();
    let ds = PgDraft(pool.clone());
    let draft = draft::create(
        &ds,
        &a,
        &c,
        draft::Create {
            request_id: KEY.into(),
            data,
        },
    )
    .await
    .unwrap();
    let original = draft::issue(
        &ds,
        &a,
        &c,
        &draft.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: draft.revision,
        },
    )
    .await
    .unwrap();
    let s = PgCredit(pool.clone());
    let mut v = prepare(&s, &a, &c, &original.id).await;
    assert_eq!(v.totals.currency, "EUR");
    assert_eq!(v.totals.payable, "-121.00");
    assert_eq!(
        v.source
            .data
            .calculation
            .exchange_rate
            .as_ref()
            .unwrap()
            .rate,
        "25"
    );
    v = credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Issue))
        .await
        .unwrap();
    assert!(s.get(&a, &other, &v.id).await.is_err());
    let pdf = s.pdf(&a, &c, &v.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let tax = faktura_infrastructure::tax_report::PgTaxReport(pool.clone())
        .report(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(tax.output, "0.00");
    assert_eq!(tax.turnover, "0.00");
    assert!(tax
        .items
        .iter()
        .any(|i| i.kind == "credit_note" && i.amount == "-525.00"));
    let refund = |method: &str| Action::Refund {
        amount: "40".into(),
        date: v.data.issued_on,
        method: method.into(),
        reason: "Synthetic refund".into(),
    };
    assert!(
        credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, refund("cash")))
            .await
            .is_err()
    );
    let action = refund("bank_transfer");
    let command = cmd(v.revision, action);
    let replay: Command = serde_json::from_value(serde_json::to_value(&command).unwrap()).unwrap();
    v = credit_note::command(&s, &a, &c, &v.id, command)
        .await
        .unwrap();
    assert_eq!(v.remaining, "81.00");
    let repeated = credit_note::command(&s, &a, &c, &v.id, replay)
        .await
        .unwrap();
    assert_eq!(repeated.events.len(), v.events.len());
    let dash = faktura_infrastructure::dashboard::PgDashboard(pool.clone())
        .dashboard(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(dash.sales.gross, "0.00");
    assert_eq!(dash.sales.outstanding, "1000.00");
    let event_id = v.events.last().unwrap().id.clone();
    v = credit_note::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Reverse {
                event_id,
                reason: "Synthetic reversal".into(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(v.remaining, "121.00");
    v = credit_note::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Offset))
        .await
        .unwrap();
    assert_eq!(v.remaining, "0.00");
    assert_eq!(v.offset, "121.00");
    let dash = faktura_infrastructure::dashboard::PgDashboard(pool.clone())
        .dashboard(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(dash.sales.outstanding, "0.00");
    assert_eq!(
        ds.get(&a, &c, &original.id)
            .await
            .unwrap()
            .settlement
            .unwrap()
            .outstanding,
        "0.00"
    );
    assert_eq!(s.pdf(&a, &c, &v.id).await.unwrap(), pdf);
}

#[path = "support/historical_documents.rs"]
mod historical_documents;
use faktura_application::{
    auth, company,
    sales::{self, Create, Data, Kind},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, sales::PgSales};
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
fn create(kind: Kind, key: &str) -> Create {
    Create {
        request_id: key.into(),
        kind,
        data: data(),
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

use faktura_application::advance::{self, AdvanceStore, Register, Reverse};
use faktura_infrastructure::advance::PgAdvance;
fn payment(revision: i64, key: &str, amount: &str) -> Register {
    Register {
        revision,
        request_id: key.into(),
        payment: faktura_domain::payment::PaymentInput {
            amount: amount.into(),
            paid_on: "2026-10-11".parse().unwrap(),
            method: "bank_transfer".into(),
            note: "Synthetic payment".into(),
        },
    }
}
use faktura_application::tax_receipt::{self, Issue};
use faktura_infrastructure::tax_receipt::PgTaxReceipt;
fn issue(d: &str, p: &str, revision: i64, key: &str) -> Issue {
    Issue {
        request_id: key.into(),
        revision,
        advance_id: d.into(),
        payment_id: p.into(),
        issued_on: "2026-10-12".parse().unwrap(),
        description: "Synthetic advance payment".into(),
        language: "cs".into(),
    }
}
async fn paid(pool: &PgPool, a: &str, c: &str) -> (String, String) {
    let d = sales::create(&PgSales(pool.clone()), a, c, create(Kind::Advance, KEY))
        .await
        .unwrap();
    advance::register(
        &PgAdvance(pool.clone()),
        a,
        c,
        &d.id,
        payment(1, KEY, "1210"),
    )
    .await
    .unwrap();
    let p = PgAdvance(pool.clone())
        .payments(a, c, &d.id)
        .await
        .unwrap()
        .remove(0);
    (d.id, p.id)
}
use faktura_application::{
    advance_settlement::{self, Configure, Selection, SettlementStore},
    draft::{self, DraftStore},
};
use faktura_infrastructure::{advance_settlement::PgSettlement, draft::PgDraft};
fn configure(key: &str, d: &str, revision: i64) -> Configure {
    Configure {
        request_id: key.into(),
        invoice_id: None,
        revision: None,
        advances: vec![Selection {
            id: d.into(),
            revision,
        }],
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn stored_tax_deductions_issue_pdf_replay_and_scope(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let (d, p) = paid(&pool, &a, &c).await;
    let tax = tax_receipt::issue(&PgTaxReceipt(pool.clone()), &a, &c, issue(&d, &p, 2, KEY))
        .await
        .unwrap();
    let s = PgSettlement(pool.clone());
    let (x, y) = tokio::join!(
        advance_settlement::configure(&s, &a, &c, configure(KEY, &d, 3)),
        advance_settlement::configure(&s, &a, &c, configure(KEY, &d, 3))
    );
    let mut v = x.unwrap();
    assert_eq!(v.id, y.unwrap().id);
    assert!(v.number.is_none());
    assert_eq!(v.totals.as_ref().unwrap().payable, "0.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.vat, "0.00");
    assert_eq!(
        v.advances.as_ref().unwrap().sources[0].tax_receipts[0].totals,
        tax.totals
    );
    assert!(s
        .candidates(&a, &c, Default::default())
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(s
        .preview(&a, &other, &v.id, v.data.calculation.clone())
        .await
        .is_err());
    assert!(
        advance_settlement::configure(&s, &a, &other, configure(KEY, &d, 3))
            .await
            .is_err()
    );
    assert!(advance_settlement::configure(
        &s,
        &a,
        &c,
        configure("00000000-0000-4000-8000-000000000002", &d, 4)
    )
    .await
    .is_err());
    // New final supply: 121 gross surcharge. Original tax deduction stays 1000+210.
    let mut line = v.data.calculation.lines[0].clone();
    line.id = "extra".into();
    line.unit_price = "100".into();
    line.quantity = "1".into();
    v.data.calculation.lines.push(line);
    v = draft::update(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Update {
            revision: v.revision,
            data: v.data.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(v.totals.as_ref().unwrap().payable, "121.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.net, "100.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.vat, "21.00");
    let issued = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: v.revision,
        },
    )
    .await
    .unwrap();
    assert!(issued.number.is_some());
    let pdf = PgDraft(pool.clone()).pdf(&a, &c, &v.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let summary = draft::get(&PgDraft(pool.clone()), &a, &c, &v.id)
        .await
        .unwrap()
        .settlement
        .unwrap();
    assert_eq!(summary.outstanding, "121.00");
    let payments = faktura_infrastructure::payment::PgPayment(pool.clone());
    let result = faktura_application::payment::register(
        &payments,
        &a,
        &c,
        &v.id,
        faktura_application::payment::Register {
            request_id: KEY.into(),
            revision: summary.revision,
            payment: faktura_domain::payment::PaymentInput {
                paid_on: issued.data.issued_on.unwrap(),
                ..payment(1, KEY, "121").payment
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(result.summary.status, "paid");
    assert_eq!(result.summary.paid, "121.00");
    assert_eq!(
        faktura_application::payment::overview(&payments, &a, &c)
            .await
            .unwrap()
            .received,
        "121.00"
    );
    // Source advances are not duplicated as regular invoice income.

    assert!(advance_settlement::configure(
        &s,
        &a,
        &c,
        Configure {
            request_id: "00000000-0000-4000-8000-000000000003".into(),
            invoice_id: Some(v.id.clone()),
            revision: Some(issued.revision),
            advances: vec![]
        }
    )
    .await
    .is_err());
    assert_eq!(PgDraft(pool.clone()).pdf(&a, &c, &v.id).await.unwrap(), pdf);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn release_race_blocks_new_payments_tax_and_wrong_customer(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let (d, p) = paid(&pool, &a, &c).await;
    let s = PgSettlement(pool.clone());
    let (x, y) = tokio::join!(
        advance_settlement::configure(&s, &a, &c, configure(KEY, &d, 2)),
        advance_settlement::configure(
            &s,
            &a,
            &c,
            configure("00000000-0000-4000-8000-000000000002", &d, 2)
        )
    );
    assert_ne!(x.is_ok(), y.is_ok());
    let v = x.or(y).unwrap();
    assert_eq!(v.totals.as_ref().unwrap().payable, "0.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.vat, "210.00"); // no DDPP: tax remains in final invoice
    assert!(advance::reverse(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &d,
        &p,
        Reverse {
            revision: 3,
            reason: "Refund".into()
        }
    )
    .await
    .is_err());
    assert!(advance::register(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &d,
        payment(3, "00000000-0000-4000-8000-000000000003", "1")
    )
    .await
    .is_err());
    assert!(
        tax_receipt::issue(&PgTaxReceipt(pool.clone()), &a, &c, issue(&d, &p, 3, KEY))
            .await
            .is_err()
    );
    let mut changed = v.data.clone();
    changed.customer.as_mut().unwrap().name = "Wrong customer".into();
    assert!(draft::update(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Update {
            revision: 1,
            data: changed
        }
    )
    .await
    .is_err());
    let release = Configure {
        request_id: "00000000-0000-4000-8000-000000000003".into(),
        invoice_id: Some(v.id.clone()),
        revision: Some(v.revision),
        advances: vec![],
    };
    let v = advance_settlement::configure(&s, &a, &c, release)
        .await
        .unwrap();
    assert!(v.advances.is_none());
    assert_eq!(v.totals.as_ref().unwrap().payable, "1210.00");
    let available = s.candidates(&a, &c, Default::default()).await.unwrap();
    assert_eq!(available.total, 1);
    let rev = available.items[0].revision;
    let zero = advance_settlement::configure(
        &s,
        &a,
        &c,
        configure("00000000-0000-4000-8000-000000000004", &d, rev),
    )
    .await
    .unwrap();
    let issued = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &zero.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: zero.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        draft::get(&PgDraft(pool.clone()), &a, &c, &issued.id)
            .await
            .unwrap()
            .settlement
            .unwrap()
            .status,
        "paid"
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(s
        .preview(&a, &c, &issued.id, issued.data.calculation.clone())
        .await
        .is_ok());
    assert!(
        advance_settlement::configure(&s, &a, &c, configure(KEY, &d, 2))
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn multiple_advances_and_unpaid_rejection(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let (d, _) = paid(&pool, &a, &c).await;
    let s = PgSettlement(pool.clone());
    let key2 = "00000000-0000-4000-8000-000000000002";
    let second = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Advance, key2))
        .await
        .unwrap();
    assert!(
        advance_settlement::configure(&s, &a, &c, configure(KEY, &second.id, 1))
            .await
            .is_err()
    );
    advance::register(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &second.id,
        payment(1, key2, "1210"),
    )
    .await
    .unwrap();
    let mut cmd = configure(KEY, &d, 2);
    cmd.advances.push(Selection {
        id: second.id.clone(),
        revision: 2,
    });
    let v = advance_settlement::configure(&s, &a, &c, cmd)
        .await
        .unwrap();
    assert_eq!(v.advances.as_ref().unwrap().credited, "2420.00");
    assert_eq!(v.data.calculation.lines.len(), 2);
    assert_eq!(v.totals.as_ref().unwrap().payable, "0.00");
    let mut reduced = v.data.clone();
    reduced.calculation.lines.pop();
    assert!(draft::update(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Update {
            revision: v.revision,
            data: reduced
        }
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn settlement_rest_mcp(pool: PgPool) {
    use axum::{body::Body, http::Request};
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt;
    let (a, token, c, _) = setup(&pool).await;
    let (d, _) = paid(&pool, &a, &c).await;
    let app = faktura_server::advance_settlement::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/advance-settlements"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token.clone());
    let (ci, si) = tokio::io::duplex(65536);
    let task =
        tokio::spawn(async move { server.serve(si).await.unwrap().waiting().await.unwrap() });
    let client = ().serve(ci).await.unwrap();
    let mut args = serde_json::to_value(configure(KEY, &d, 2))
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    args.insert("company_id".into(), c.clone().into());
    let r = client
        .call_tool(CallToolRequestParams::new("settlement_configure").with_arguments(args))
        .await
        .unwrap();
    assert_ne!(r.is_error, Some(true));
    let app = faktura_server::advance_settlement::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let command = serde_json::to_vec(&configure(KEY, &d, 2)).unwrap();
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/advance-settlements"))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .body(Body::from(command.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), 401);
    let replay = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/advance-settlements"))
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(command))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay.status(), 200);
    let rest: faktura_application::draft::Draft = serde_json::from_slice(
        &axum::body::to_bytes(replay.into_body(), 1_000_000)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(rest.totals.as_ref().unwrap().payable, "0.00");
    assert_eq!(
        PgSettlement(pool.clone())
            .candidates(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade_019_preserves_invoices_and_paid_advances(pool: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610120020)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let d = historical_documents::sales(&pool, &a, &c, create(Kind::Advance, KEY)).await;
    // Seed the previous version's payment schema, before the new reservation guard exists.
    sqlx::query("INSERT INTO advance_payment(company_id,document_id,actor_id,request_id,command,amount,paid_on,method,note) VALUES($1,$2,$3,$4::text::uuid,'{}',1210,'2026-10-11','bank_transfer','Upgrade fixture')")
        .bind(c.parse::<i64>().unwrap()).bind(d.id.parse::<i64>().unwrap()).bind(a.parse::<i64>().unwrap()).bind(KEY).execute(&pool).await.unwrap();
    sqlx::query(
        "UPDATE sales_document SET advance_paid=1210,revision=2 WHERE company_id=$1 AND id=$2",
    )
    .bind(c.parse::<i64>().unwrap())
    .bind(d.id.parse::<i64>().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let normal = draft::create(
        &PgDraft(pool.clone()),
        &a,
        &c,
        draft::Create {
            request_id: KEY.into(),
            data: data().invoice,
        },
    )
    .await
    .unwrap();
    let pdf: Vec<u8> =
        sqlx::query_scalar("SELECT advance_pdf FROM sales_document WHERE id=$1::text::bigint")
            .bind(&d.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    faktura_infrastructure::migrate(&pool).await.unwrap();
    assert_eq!(
        pdf,
        PgAdvance(pool.clone()).pdf(&a, &c, &d.id).await.unwrap()
    );
    let restored = PgDraft(pool.clone()).get(&a, &c, &normal.id).await.unwrap();
    assert!(restored.advances.is_none());
    assert_eq!(
        serde_json::to_value(restored.totals).unwrap(),
        serde_json::to_value(normal.totals).unwrap()
    );
    let prepared = advance_settlement::configure(
        &PgSettlement(pool.clone()),
        &a,
        &c,
        configure("00000000-0000-4000-8000-000000000008", &d.id, 2),
    )
    .await
    .unwrap();
    let totals = prepared.totals.unwrap();
    assert_eq!(totals.payable, "0.00");
    assert_eq!(totals.amounts.vat, "210.00");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn nonpayer_rounded_advance_preserves_payable(pool: PgPool) {
    use faktura_domain::invoice::{Rounding, TaxMode};
    let (a, _, _, _) = setup(&pool).await;
    let c = company::create(
        &PgCompany(pool.clone()),
        &a,
        "00000000-0000-4000-8000-000000000009",
        CompanySettings::default(),
    )
    .await
    .unwrap()
    .id;
    let mut source = create(Kind::Advance, KEY);
    source.data.invoice.calculation.tax_mode = TaxMode::NonPayer;
    source.data.invoice.calculation.lines[0].vat_percent = "0".into();
    source.data.invoice.calculation.lines[0].unit_price = "1000.60".into();
    source.data.invoice.calculation.rounding = Rounding::WholeCzk;
    let d = sales::create(&PgSales(pool.clone()), &a, &c, source)
        .await
        .unwrap();
    assert_eq!(d.totals.as_ref().unwrap().payable, "1001.00");
    advance::register(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &d.id,
        payment(1, KEY, "1001"),
    )
    .await
    .unwrap();
    let v = advance_settlement::configure(
        &PgSettlement(pool.clone()),
        &a,
        &c,
        configure(KEY, &d.id, 2),
    )
    .await
    .unwrap();
    assert_eq!(v.totals.as_ref().unwrap().payable, "0.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.vat, "0.00");
    assert_eq!(v.advances.as_ref().unwrap().rounding_adjustment, "0.40");
    let issued = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: v.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(issued.totals.as_ref().unwrap().payable, "0.00");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_advance_tax_settlement_snapshot_and_payment(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let mut input = create(Kind::Advance, KEY);
    input.data.invoice.payment.method = "bank_transfer".into();
    input.data.invoice.payment.bank_account = "12345678/0100".into();
    input.data.invoice.calculation.currency = "EUR".into();
    input.data.invoice.calculation.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "25".into(),
        units: "1".into(),
    });
    let doc = sales::create(&PgSales(pool.clone()), &a, &c, input)
        .await
        .unwrap();
    let mut cash = payment(1, KEY, "1210");
    cash.payment.method = "cash".into();
    assert!(
        advance::register(&PgAdvance(pool.clone()), &a, &c, &doc.id, cash)
            .await
            .is_err()
    );
    advance::register(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &doc.id,
        payment(1, KEY, "1210"),
    )
    .await
    .unwrap();
    let p = PgAdvance(pool.clone())
        .payments(&a, &c, &doc.id)
        .await
        .unwrap()
        .remove(0)
        .id;
    let d = doc.id;
    let tax = tax_receipt::issue(&PgTaxReceipt(pool.clone()), &a, &c, issue(&d, &p, 2, KEY))
        .await
        .unwrap();
    let s = PgSettlement(pool.clone());
    let (x, y) = tokio::join!(
        advance_settlement::configure(&s, &a, &c, configure(KEY, &d, 3)),
        advance_settlement::configure(&s, &a, &c, configure(KEY, &d, 3))
    );
    let mut v = x.unwrap();
    assert_eq!(v.id, y.unwrap().id);
    assert!(v.number.is_none());
    assert_eq!(tax.totals.currency, "EUR");
    assert_eq!(v.advances.as_ref().unwrap().currency, "EUR");
    let mut different = v.data.calculation.clone();
    different.exchange_rate.as_mut().unwrap().rate = "26".into();
    assert!(s.preview(&a, &c, &v.id, different.clone()).await.is_err());
    let mut invalid = v.data.clone();
    invalid.calculation = different;
    assert!(draft::update(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Update {
            revision: v.revision,
            data: invalid
        }
    )
    .await
    .is_err());
    assert_eq!(v.totals.as_ref().unwrap().payable, "0.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.vat, "0.00");
    assert_eq!(
        v.advances.as_ref().unwrap().sources[0].tax_receipts[0].totals,
        tax.totals
    );
    assert!(s
        .candidates(&a, &c, Default::default())
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(s
        .preview(&a, &other, &v.id, v.data.calculation.clone())
        .await
        .is_err());
    assert!(
        advance_settlement::configure(&s, &a, &other, configure(KEY, &d, 3))
            .await
            .is_err()
    );
    assert!(advance_settlement::configure(
        &s,
        &a,
        &c,
        configure("00000000-0000-4000-8000-000000000002", &d, 4)
    )
    .await
    .is_err());
    // New final supply: 121 gross surcharge. Original tax deduction stays 1000+210.
    let mut line = v.data.calculation.lines[0].clone();
    line.id = "extra".into();
    line.unit_price = "100".into();
    line.quantity = "1".into();
    v.data.calculation.lines.push(line);
    v = draft::update(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Update {
            revision: v.revision,
            data: v.data.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(v.totals.as_ref().unwrap().payable, "121.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.net, "100.00");
    assert_eq!(v.totals.as_ref().unwrap().amounts.vat, "21.00");
    let issued = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &v.id,
        draft::Issue {
            request_id: KEY.into(),
            revision: v.revision,
        },
    )
    .await
    .unwrap();
    assert!(issued.number.is_some());
    let pdf = PgDraft(pool.clone()).pdf(&a, &c, &v.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let summary = draft::get(&PgDraft(pool.clone()), &a, &c, &v.id)
        .await
        .unwrap()
        .settlement
        .unwrap();
    assert_eq!(summary.outstanding, "121.00");
    let payments = faktura_infrastructure::payment::PgPayment(pool.clone());
    let result = faktura_application::payment::register(
        &payments,
        &a,
        &c,
        &v.id,
        faktura_application::payment::Register {
            request_id: KEY.into(),
            revision: summary.revision,
            payment: faktura_domain::payment::PaymentInput {
                paid_on: issued.data.issued_on.unwrap(),
                ..payment(1, KEY, "121").payment
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(result.summary.status, "paid");
    assert_eq!(result.summary.paid, "121.00");
    assert_eq!(
        faktura_application::payment::overview(&payments, &a, &c)
            .await
            .unwrap()
            .received,
        "3025.00"
    );
    use faktura_application::tax_report::TaxReportStore;
    let report = faktura_infrastructure::tax_report::PgTaxReport(pool.clone())
        .report(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(report.output, "5775.00");
    assert_eq!(report.turnover, "27500.00");
    // Source advances are not duplicated as regular invoice income.

    assert!(advance_settlement::configure(
        &s,
        &a,
        &c,
        Configure {
            request_id: "00000000-0000-4000-8000-000000000003".into(),
            invoice_id: Some(v.id.clone()),
            revision: Some(issued.revision),
            advances: vec![]
        }
    )
    .await
    .is_err());
    assert_eq!(PgDraft(pool.clone()).pdf(&a, &c, &v.id).await.unwrap(), pdf);
}

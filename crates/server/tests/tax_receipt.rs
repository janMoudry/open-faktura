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
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let second = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        "00000000-0000-4000-8000-000000000002",
        CompanySettings::default(),
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
use faktura_application::tax_receipt::{self, Issue, TaxReceiptStore};
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
        payment(1, KEY, "605"),
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
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_receipt_preview_issue_replay_snapshot_scope_and_reversal(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let (d, p) = paid(&pool, &a, &c).await;
    let s = PgTaxReceipt(pool.clone());
    let preview = s.preview(&a, &c, &d, &p).await.unwrap();
    assert_eq!(preview.totals.amounts.net, "500.00");
    assert_eq!(preview.totals.amounts.vat, "105.00");
    assert!(preview.number.is_empty());
    let (x, y) = tokio::join!(
        tax_receipt::issue(&s, &a, &c, issue(&d, &p, 2, KEY)),
        tax_receipt::issue(&s, &a, &c, issue(&d, &p, 2, KEY))
    );
    let doc = x.unwrap();
    assert_eq!(doc.id, y.unwrap().id);
    assert_eq!(doc.number, "DDPP20260001");
    let pdf = s.pdf(&a, &c, &doc.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(s.get(&a, &other, &doc.id).await.is_err());
    assert!(s.pdf(&a, &other, &doc.id).await.is_err());
    assert!(s.preview(&a, &other, &d, &p).await.is_err());
    assert!(tax_receipt::issue(
        &s,
        &a,
        &c,
        issue(&d, &p, 3, "00000000-0000-4000-8000-000000000002")
    )
    .await
    .is_err());
    let mut changed = issue(&d, &p, 2, KEY);
    changed.description = "changed".into();
    assert!(tax_receipt::issue(&s, &a, &c, changed).await.is_err());
    assert!(advance::reverse(
        &PgAdvance(pool.clone()),
        &a,
        &c,
        &d,
        &p,
        Reverse {
            revision: 3,
            reason: "Correction".into()
        }
    )
    .await
    .is_err());
    let receipt = PgAdvance(pool.clone())
        .payments(&a, &c, &d)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(receipt.tax_receipt_id, Some(doc.id.clone()));
    assert!(!receipt.reversed);
    let page = tax_receipt::list(
        &s,
        &a,
        &c,
        tax_receipt::Search {
            query: "DDPP".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.total, 1);
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{name}','\"Changed company\"') WHERE id=$1").bind(c.parse::<i64>().unwrap()).execute(&pool).await.unwrap();
    assert_eq!(s.pdf(&a, &c, &doc.id).await.unwrap(), pdf);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_payment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    // Viewer can read but cannot issue. Installation admin still obeys profile membership.
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1 AND user_id=$2")
        .bind(c.parse::<i64>().unwrap())
        .bind(a.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(s.get(&a, &c, &doc.id).await.is_ok());
    assert!(tax_receipt::issue(&s, &a, &c, issue(&d, &p, 2, KEY))
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_receipt_invalid_and_issue_reverse_race(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let (d, p) = paid(&pool, &a, &c).await;
    let s = PgTaxReceipt(pool.clone());
    let mut cmd = issue(&d, &p, 2, KEY);
    cmd.issued_on = "2026-10-10".parse().unwrap();
    assert!(tax_receipt::issue(&s, &a, &c, cmd).await.is_err());
    let mut cmd = issue(&d, &p, 1, KEY);
    assert!(tax_receipt::issue(&s, &a, &c, cmd.clone()).await.is_err());
    cmd.revision = 2;
    cmd.language = "xx".into();
    assert!(tax_receipt::issue(&s, &a, &c, cmd).await.is_err());
    let advance = PgAdvance(pool.clone());
    let (x, y) = tokio::join!(
        tax_receipt::issue(&s, &a, &c, issue(&d, &p, 2, KEY)),
        advance::reverse(
            &advance,
            &a,
            &c,
            &d,
            &p,
            Reverse {
                revision: 2,
                reason: "Correction".into()
            }
        )
    );
    assert_ne!(x.is_ok(), y.is_ok());
    // A non-payer advance cannot become a tax document.
    let mut cmd = create(Kind::Advance, "00000000-0000-4000-8000-000000000002");
    cmd.data.invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    cmd.data.invoice.calculation.lines[0].vat_percent = "0".into();
    let d = sales::create(&PgSales(pool.clone()), &a, &c, cmd)
        .await
        .unwrap();
    advance::register(
        &advance,
        &a,
        &c,
        &d.id,
        payment(1, "00000000-0000-4000-8000-000000000002", "500"),
    )
    .await
    .unwrap();
    let p = advance.payments(&a, &c, &d.id).await.unwrap().remove(0);
    assert!(s.preview(&a, &c, &d.id, &p.id).await.is_err());
    assert!(tax_receipt::issue(
        &s,
        &a,
        &c,
        issue(&d.id, &p.id, 2, "00000000-0000-4000-8000-000000000002")
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_receipt_rest_mcp(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt;
    let (a, token, c, _) = setup(&pool).await;
    let (d, p) = paid(&pool, &a, &c).await;
    let app = faktura_server::tax_receipt::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let uri = format!("/api/companies/{c}/advances/{d}/payments/{p}/tax-preview");
    for (cookie, code) in [("", 401), (token.as_str(), 200)] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&uri)
                    .header("cookie", format!("faktura_session={cookie}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), code);
    }
    let (server, client) = tokio::io::duplex(65536);
    let handler = faktura_server::mcp::CompanyMcp::new(pool.clone(), token.clone());
    let service = tokio::spawn(async move {
        handler
            .serve(server)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap()
    });
    let client = ().serve(client).await.unwrap();
    let mut args = serde_json::to_value(issue(&d, &p, 2, KEY))
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    args.insert("company_id".into(), serde_json::json!(c));
    let result = client
        .call_tool(CallToolRequestParams::new("tax_receipt_issue").with_arguments(args))
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let doc = PgTaxReceipt(pool.clone())
        .list(&a, &c, Default::default())
        .await
        .unwrap()
        .items
        .remove(0);
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/tax-receipts/{}/pdf", doc.id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["content-type"], "application/pdf");
    assert!(to_bytes(r.into_body(), 10_000_000)
        .await
        .unwrap()
        .starts_with(b"%PDF"));
    client.cancel().await.unwrap();
    service.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn tax_receipt_upgrade_from_018(pool: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610120019)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let d = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Advance, KEY))
        .await
        .unwrap();
    sqlx::query("INSERT INTO advance_payment(company_id,document_id,actor_id,request_id,command,amount,paid_on,method,note) VALUES($1,$2,$3,$4::text::uuid,'{}',605,'2026-10-11','bank_transfer','Upgrade fixture')").bind(c.parse::<i64>().unwrap()).bind(d.id.parse::<i64>().unwrap()).bind(a.parse::<i64>().unwrap()).bind(KEY).execute(&pool).await.unwrap();
    sqlx::query(
        "UPDATE sales_document SET advance_paid=605,revision=2 WHERE company_id=$1 AND id=$2",
    )
    .bind(c.parse::<i64>().unwrap())
    .bind(d.id.parse::<i64>().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let before = PgAdvance(pool.clone()).pdf(&a, &c, &d.id).await.unwrap();
    faktura_infrastructure::migrate(&pool).await.unwrap();
    assert_eq!(
        before,
        PgAdvance(pool.clone()).pdf(&a, &c, &d.id).await.unwrap()
    );
    let p = PgAdvance(pool.clone())
        .payments(&a, &c, &d.id)
        .await
        .unwrap()
        .remove(0);
    assert!(tax_receipt::issue(
        &PgTaxReceipt(pool.clone()),
        &a,
        &c,
        issue(&d.id, &p.id, 2, KEY)
    )
    .await
    .is_ok());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn multiple_tax_receipts_cannot_exceed_advance(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let (d, p) = paid(&pool, &a, &c).await;
    let s = PgTaxReceipt(pool.clone());
    let payments = PgAdvance(pool.clone());
    tax_receipt::issue(&s, &a, &c, issue(&d, &p, 2, KEY))
        .await
        .unwrap();
    let second = "00000000-0000-4000-8000-000000000002";
    advance::register(&payments, &a, &c, &d, payment(3, second, "605"))
        .await
        .unwrap();
    let p = payments.payments(&a, &c, &d).await.unwrap().remove(0);
    let doc = tax_receipt::issue(&s, &a, &c, issue(&d, &p.id, 4, second))
        .await
        .unwrap();
    assert_eq!(doc.number, "DDPP20260002");
    assert_eq!(doc.totals.amounts.net, "500.00");
    let third = "00000000-0000-4000-8000-000000000003";
    advance::register(&payments, &a, &c, &d, payment(5, third, "1"))
        .await
        .unwrap();
    let p = payments.payments(&a, &c, &d).await.unwrap().remove(0);
    assert!(tax_receipt::issue(&s, &a, &c, issue(&d, &p.id, 6, third))
        .await
        .is_err());
    assert_eq!(s.list(&a, &c, Default::default()).await.unwrap().total, 2);
    // Failed issuance did not consume a number or change the source revision.
    assert_eq!(
        sales::get(&PgSales(pool.clone()), &a, &c, &d)
            .await
            .unwrap()
            .revision,
        6
    );
}

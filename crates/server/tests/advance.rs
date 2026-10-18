#[path = "support/historical_documents.rs"]
mod historical_documents;
use faktura_application::{
    auth, company,
    sales::{self, Convert, Create, Data, Kind, Search, Update},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, sales::PgSales};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> Data {
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.tax_on = None;
    invoice.calculation.lines[0].unit_price = "1200".into();
    invoice.calculation.lines[0].quantity = "2".into();
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
fn conversion(revision: i64, key: &str) -> Convert {
    Convert {
        request_id: key.into(),
        revision,
        issued_on: "2026-10-11".parse().unwrap(),
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
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn advance_numbering_payment_replay_reversal_and_scope(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let s = PgSales(pool.clone());
    let p = PgAdvance(pool.clone());
    let (x, y) = tokio::join!(
        sales::create(&s, &a, &c, create(Kind::Advance, KEY)),
        sales::create(&s, &a, &c, create(Kind::Advance, KEY))
    );
    let v = x.unwrap();
    assert_eq!(v.id, y.unwrap().id);
    assert_eq!(v.number.as_deref(), Some("ZF20260001"));
    assert_eq!(v.data.invoice.payment.variable_symbol, "20260001");
    let pdf = p.pdf(&a, &c, &v.id).await.unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(p.pdf(&a, &other, &v.id).await.is_err());
    assert!(sales::convert(&s, &a, &c, &v.id, conversion(1, KEY))
        .await
        .is_err());
    let quote = sales::create(
        &s,
        &a,
        &c,
        create(Kind::Quote, "00000000-0000-4000-8000-000000000002"),
    )
    .await
    .unwrap();
    assert_eq!(quote.number.as_deref(), Some("CN20260001"));
    assert!(p.pdf(&a, &c, &quote.id).await.is_err());
    let (x, y) = tokio::join!(
        advance::register(&p, &a, &c, &v.id, payment(1, KEY, "1000")),
        advance::register(&p, &a, &c, &v.id, payment(1, KEY, "1000"))
    );
    x.unwrap();
    y.unwrap();
    assert_eq!(p.payments(&a, &c, &v.id).await.unwrap().len(), 1);
    let paid = sales::get(&s, &a, &c, &v.id).await.unwrap();
    assert_eq!(paid.advance_paid.as_deref(), Some("1000.00"));
    assert_eq!(paid.revision, 2);
    assert!(
        advance::register(&p, &a, &c, &v.id, payment(2, KEY, "1001"))
            .await
            .is_err()
    );
    assert!(sales::update(
        &s,
        &a,
        &c,
        &v.id,
        Update {
            revision: 2,
            archived: false,
            data: paid.data.clone()
        }
    )
    .await
    .is_err());
    advance::register(
        &p,
        &a,
        &c,
        &v.id,
        payment(2, "00000000-0000-4000-8000-000000000003", "1500"),
    )
    .await
    .unwrap();
    assert_eq!(
        sales::list(
            &s,
            &a,
            &c,
            Search {
                kind: Kind::Advance,
                status: "paid".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    assert_eq!(p.pdf(&a, &c, &v.id).await.unwrap(), pdf);
    let receipt = p.payments(&a, &c, &v.id).await.unwrap().remove(0);
    advance::reverse(
        &p,
        &a,
        &c,
        &v.id,
        &receipt.id,
        Reverse {
            revision: 3,
            reason: "Correction".into(),
        },
    )
    .await
    .unwrap();
    advance::reverse(
        &p,
        &a,
        &c,
        &v.id,
        &receipt.id,
        Reverse {
            revision: 3,
            reason: "Correction".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        sales::get(&s, &a, &c, &v.id)
            .await
            .unwrap()
            .advance_paid
            .as_deref(),
        Some("1000.00")
    );
    assert_eq!(
        sales::list(
            &s,
            &a,
            &c,
            Search {
                kind: Kind::Advance,
                status: "outstanding".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_payment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(p.pdf(&a, &c, &v.id).await.is_ok());
    assert!(advance::register(
        &p,
        &a,
        &c,
        &v.id,
        payment(4, "00000000-0000-4000-8000-000000000004", "1")
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn advance_rest_mcp_and_validation(pool: PgPool) {
    let (a, token, c, _) = setup(&pool).await;
    let s = PgSales(pool.clone());
    let mut invalid = create(Kind::Advance, KEY);
    invalid.data.invoice.tax_on = Some("2026-10-11".parse().unwrap());
    assert!(sales::create(&s, &a, &c, invalid).await.is_err());
    let v = sales::create(&s, &a, &c, create(Kind::Advance, KEY))
        .await
        .unwrap();
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let app =
        faktura_server::advance::router(pool.clone(), vec!["http://localhost:5173".into()], false);
    for (cookie, code) in [("", 401), (token.as_str(), 200)] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/companies/{c}/advances/{}/pdf", v.id))
                    .header("cookie", format!("faktura_session={cookie}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), code);
        if code == 200 {
            assert_eq!(r.headers()["cache-control"], "no-store");
            assert!(to_bytes(r.into_body(), 10000000)
                .await
                .unwrap()
                .starts_with(b"%PDF"));
        }
    }
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token);
    let (ci, si) = tokio::io::duplex(65536);
    let task =
        tokio::spawn(async move { server.serve(si).await.unwrap().waiting().await.unwrap() });
    let client = ().serve(ci).await.unwrap();
    let mut args = serde_json::to_value(payment(1, KEY, "2400"))
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    args.insert("company_id".into(), c.clone().into());
    args.insert("id".into(), v.id.clone().into());
    let r = client
        .call_tool(CallToolRequestParams::new("advance_register").with_arguments(args))
        .await
        .unwrap();
    assert_eq!(r.is_error, Some(false));
    client.cancel().await.unwrap();
    task.await.unwrap();
    assert_eq!(
        sales::get(&s, &a, &c, &v.id)
            .await
            .unwrap()
            .advance_paid
            .as_deref(),
        Some("2400.00")
    );
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade_from_017_preserves_quote(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|m| m.version < 202610120018)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    old.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let s = PgSales(pool.clone());
    let q = historical_documents::sales(&pool, &a, &c, create(Kind::Quote, KEY)).await;
    all.run(&pool).await.unwrap();
    let restored = sales::get(&s, &a, &c, &q.id).await.unwrap();
    assert_eq!(restored.number, q.number);
    assert_eq!(restored.data.name, q.data.name);
    assert!(sales::create(
        &s,
        &a,
        &c,
        create(Kind::Advance, "00000000-0000-4000-8000-000000000009")
    )
    .await
    .is_ok());
}

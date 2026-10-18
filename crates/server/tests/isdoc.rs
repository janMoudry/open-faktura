//! Synthetic snapshots, validated against the official 6.0.2 XSD, not a target-app acceptance claim.
use faktura_application::{draft::Draft, isdoc::render};
use faktura_domain::invoice;
fn sample() -> Draft {
    let data: invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    let totals = invoice::calculate(&data.calculation).unwrap();
    Draft {
        id: "1".into(),
        company_id: "1".into(),
        revision: 2,
        data,
        totals: Some(totals),
        number: Some("2026-001".into()),
        issued_at: Some("2026-10-11T00:00:00Z".into()),
        settlement: None,
        advances: None,
    }
}
fn validate_xml(bytes: &[u8]) {
    use std::io::Write;
    let mut child = std::process::Command::new("xmllint")
        .args([
            "--noout",
            "--nonet",
            "--schema",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/fixtures/isdoc/isdoc-invoice-6.0.2.xsd"
            ),
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("xmllint is required for the official ISDOC XSD check");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn official_xsd_decimal_snapshot_and_escaping() {
    let mut d = sample();
    d.data.supplier.name = "A & <B> \"C\"".into();
    d.data.note = "Poznámka & <note>".into();
    d.data.payment.variable_symbol = "0000123".into();
    let first = render(&d, "00000000-0000-4000-8000-000000000001").unwrap();
    validate_xml(&first.bytes);
    let xml = String::from_utf8(first.bytes.clone()).unwrap();
    assert!(xml.contains("A &amp; &lt;B&gt; &quot;C&quot;"));
    assert!(xml.contains("<LineExtensionAmount>59.99</LineExtensionAmount>"));
    assert!(xml.contains("<VariableSymbol>0000123</VariableSymbol>"));
    assert!(xml.contains("<PayableAmount>72.59</PayableAmount>"));
    d.settlement=Some(serde_json::from_value(serde_json::json!({"paid":"50.00","outstanding":"22.59","status":"partial","overpayment":"0.00","overdue":false,"revision":1,"today":"2026-10-18"})).unwrap());
    assert_eq!(
        render(&d, "00000000-0000-4000-8000-000000000001")
            .unwrap()
            .bytes,
        first.bytes
    );
    d.number = None;
    assert!(render(&d, "00000000-0000-4000-8000-000000000001").is_err());
}
#[test]
fn official_xsd_foreign_rate_units_nonpayer_and_gross() {
    let mut d = sample();
    d.data.calculation.currency = "JPY".into();
    d.data.calculation.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "15.25".into(),
        units: "100".into(),
    });
    d.totals = Some(invoice::calculate(&d.data.calculation).unwrap());
    let f = render(&d, "00000000-0000-4000-8000-000000000001").unwrap();
    validate_xml(&f.bytes);
    let xml = String::from_utf8(f.bytes).unwrap();
    assert!(xml.contains("<ForeignCurrencyCode>JPY</ForeignCurrencyCode>"));
    assert!(xml.contains("<CurrRate>15.25</CurrRate><RefCurrRate>100</RefCurrRate>"));
    assert!(xml.contains("<TaxExclusiveAmount>9.148475</TaxExclusiveAmount><TaxExclusiveAmountCurr>59.99</TaxExclusiveAmountCurr>"));
    d.data.calculation.currency = "CZK".into();
    d.data.calculation.exchange_rate = None;
    d.data.calculation.tax_mode = invoice::TaxMode::NonPayer;
    d.data.calculation.lines[0].vat_percent = "0".into();
    d.data.calculation.price_mode = invoice::PriceMode::Gross;
    d.totals = Some(invoice::calculate(&d.data.calculation).unwrap());
    let f = render(&d, "00000000-0000-4000-8000-000000000001").unwrap();
    validate_xml(&f.bytes);
    assert!(String::from_utf8(f.bytes)
        .unwrap()
        .contains("<VATApplicable>false</VATApplicable>"));
}

use faktura_application::{auth, company};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
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

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn export_authorization_rest_snapshot_and_draft_rejection(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use faktura_application::isdoc::IsdocStore;
    use tower::ServiceExt;
    let (actor, token, c, other) = setup(&pool).await;
    let d = sample();
    let id:i64=sqlx::query_scalar("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,number,issued_at,issue_request_id,issue_actor_id,issue_revision) VALUES($1,$2,$3,$4,$4,$5,'ISDOC-1',now(),$6,$2,1) RETURNING id")
 .bind(c.parse::<i64>().unwrap()).bind(actor.parse::<i64>().unwrap()).bind(uuid::Uuid::new_v4()).bind(serde_json::to_value(&d.data).unwrap()).bind(serde_json::to_value(&d.totals).unwrap()).bind(uuid::Uuid::new_v4()).fetch_one(&pool).await.unwrap();
    let store = faktura_infrastructure::isdoc::PgIsdoc(pool.clone());
    let file = store.export(&actor, &c, &id.to_string()).await.unwrap();
    validate_xml(&file.bytes);
    assert!(store.export(&actor, &other, &id.to_string()).await.is_err());
    let app =
        faktura_server::isdoc::router(pool.clone(), vec!["http://localhost:5173".into()], false);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/invoices/{id}/isdoc"))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let actual = to_bytes(response.into_body(), 2097152).await.unwrap();
    assert_eq!(actual.as_ref(), file.bytes);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/invoices/{id}/isdoc"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let draft = faktura_application::draft::create(
        &faktura_infrastructure::draft::PgDraft(pool.clone()),
        &actor,
        &c,
        faktura_application::draft::Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            data: d.data,
        },
    )
    .await
    .unwrap();
    assert!(store.export(&actor, &c, &draft.id).await.is_err());
}

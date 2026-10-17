use faktura_application::{
    auth, company,
    draft::{self, Create, Error},
};
use faktura_domain::{company::CompanySettings, invoice::DraftData};
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, draft::PgDraft};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> DraftData {
    let mut d: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    d.supplier.registration_number = "12345678".into();
    d.supplier.email = "reply@example.test".into();
    d.payment.method = "cash".into();
    d.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    d.calculation.lines[0].vat_percent = "0".into();
    d.tax_on = None;
    d
}
fn create(key: &str) -> Create {
    Create {
        request_id: key.into(),
        data: data(),
    }
}
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        pool.clone(),
        Some("synthetic-contact-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-contact-bootstrap-key-only-tests",
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
use faktura_application::export::{self, Request};
use faktura_infrastructure::export::PgExport;
fn request(
    kind: &str,
    format: &str,
    language: &str,
    columns: serde_json::Value,
    search: serde_json::Value,
) -> Request {
    serde_json::from_value(serde_json::json!({"format":format,"language":language,"columns":columns,"selection":{"kind":kind,"search":search}})).unwrap()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn invoice_filters_scope_and_formats(pool: PgPool) {
    let (actor, _, company, other) = setup(&pool).await;
    let store = PgDraft(pool.clone());
    draft::create(&store, &actor, &company, create(KEY))
        .await
        .unwrap();
    let mut second = create("00000000-0000-4000-8000-000000000003");
    second.data.order_number = "exclude-me".into();
    second.data.customer.as_mut().unwrap().name = "Another customer".into();
    draft::create(&store, &actor, &company, second)
        .await
        .unwrap();
    let export = PgExport(pool.clone());
    let f = export::run(
        &export,
        &actor,
        &company,
        request(
            "invoices",
            "csv",
            "cs",
            serde_json::json!(["description", "payable", "issued_on"]),
            serde_json::json!({"query":"exclude-me","page":7,"page_size":1}),
        ),
    )
    .await
    .unwrap();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(&f.bytes[3..]);
    assert_eq!(reader.headers().unwrap().get(1), Some("Celkem k úhradě"));
    let records = reader
        .records()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0][1].contains(','));
    assert!(records[0][2].contains('.'));
    let f = export::run(
        &export,
        &actor,
        &company,
        request(
            "invoices",
            "xlsx",
            "en",
            serde_json::json!(["description", "status"]),
            serde_json::json!({"status":"draft","page_size":1}),
        ),
    )
    .await
    .unwrap();
    use calamine::Reader;
    let mut book = calamine::Xlsx::new(std::io::Cursor::new(f.bytes)).unwrap();
    let rows = book.worksheet_range("Export").unwrap();
    assert_eq!(rows.height(), 3);
    assert_eq!(rows.get_value((1, 1)).unwrap().to_string(), "Draft");
    let f = export::run(
        &export,
        &actor,
        &other,
        request(
            "invoices",
            "csv",
            "en",
            serde_json::json!(["number"]),
            serde_json::json!({}),
        ),
    )
    .await
    .unwrap();
    assert_eq!(String::from_utf8(f.bytes).unwrap().lines().count(), 1);
    assert!(matches!(
        export::run(
            &export,
            "99999999",
            &company,
            request(
                "invoices",
                "csv",
                "en",
                serde_json::json!(["number"]),
                serde_json::json!({})
            )
        )
        .await,
        Err(Error::NotFound)
    ));
    for columns in [
        serde_json::json!([]),
        serde_json::json!(["number", "number"]),
        serde_json::json!(["password"]),
    ] {
        assert!(export::run(
            &export,
            &actor,
            &company,
            request("invoices", "csv", "en", columns, serde_json::json!({}))
        )
        .await
        .is_err());
    }
    assert!(export::run(
        &export,
        &actor,
        &company,
        request(
            "invoices",
            "csv",
            "en",
            serde_json::json!(["number"]),
            serde_json::json!({"sort":"bad"})
        )
    )
    .await
    .is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(company.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(export::run(
        &export,
        &actor,
        &company,
        request(
            "invoices",
            "csv",
            "en",
            serde_json::json!(["number"]),
            serde_json::json!({})
        )
    )
    .await
    .is_ok());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn contacts_limits_rest_and_mcp(pool: PgPool) {
    use faktura_application::contact;
    use faktura_domain::contact::ContactData;
    let (actor, token, company, _) = setup(&pool).await;
    let mut data = ContactData {
        name: "Export customer".into(),
        registration_number: "00123456".into(),
        ..Default::default()
    };
    data.billing_address.country = "CZ".into();
    data.payment_method = "bank_transfer".into();
    contact::create(
        &faktura_infrastructure::contact::PgContact(pool.clone()),
        &actor,
        &company,
        contact::Create {
            request_id: KEY.into(),
            data,
        },
    )
    .await
    .unwrap();
    let input = serde_json::json!({"format":"csv","language":"en","columns":["name","registration_number"],"selection":{"kind":"contacts","search":{"query":"Export"}}});
    use axum::{
        body::{to_bytes, Body},
        http::Request as HttpRequest,
    };
    use tower::ServiceExt;
    let app =
        faktura_server::export::router(pool.clone(), vec!["http://localhost:5173".into()], false);
    for (cookie, status) in [("", 401), (token.as_str(), 200)] {
        let r = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri(format!("/api/companies/{company}/exports"))
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("origin", "http://localhost:5173")
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(input.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), status);
        if status == 200 {
            assert_eq!(r.headers()["cache-control"], "no-store");
            let bytes = to_bytes(r.into_body(), 10000).await.unwrap();
            assert!(String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains("00123456"));
        }
    }
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token);
    let (client_io, server_io) = tokio::io::duplex(65536);
    let task = tokio::spawn(async move {
        server
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap()
    });
    let client = ().serve(client_io).await.unwrap();
    let mut args = input.as_object().unwrap().clone();
    args.insert("company_id".into(), company.clone().into());
    let r = client
        .call_tool(CallToolRequestParams::new("register_export").with_arguments(args))
        .await
        .unwrap();
    assert_eq!(r.is_error, Some(false));
    let v: serde_json::Value = serde_json::from_str(&r.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(v["filename"], "open-faktura-contacts.csv");
    client.cancel().await.unwrap();
    task.await.unwrap();
    // Seed enough rows to prove pagination is ignored and the cap fails without truncating.
    sqlx::query("INSERT INTO contact(company_id,creator_id,request_id,initial_data,data) SELECT company_id,creator_id,md5(n::text)::uuid,initial_data,data FROM contact CROSS JOIN generate_series(1,10000) n WHERE company_id=$1").bind(company.parse::<i64>().unwrap()).execute(&pool).await.unwrap();
    assert!(
        matches!(export::run(&PgExport(pool),&actor,&company,request("contacts","csv","en",serde_json::json!(["name"]),serde_json::json!({}))).await,Err(Error::Invalid(v)) if v==vec!["export_limit"])
    );
}

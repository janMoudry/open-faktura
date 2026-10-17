use faktura_application::{
    auth, company,
    period_report::{PeriodReportStore, Search},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, period_report::PgPeriodReport};
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

fn query(page: i64) -> Search {
    Search {
        from: "2026-01-01".parse().unwrap(),
        to: "2026-12-31".parse().unwrap(),
        page: Some(page),
        value_mode: faktura_application::period_report::ValueMode::Gross,
        cumulative: false,
        compare_previous: false,
    }
}
// Synthetic persisted snapshots: independent expected totals, not calculator output.
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn exact_totals_sources_pages_and_profile_isolation(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let c_id: i64 = c.parse().unwrap();
    let a_id: i64 = a.parse().unwrap();
    let invoice:i64=sqlx::query_scalar("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,number,issued_at,issue_request_id,issue_actor_id,issue_revision,advances) VALUES($1,$2,$3,'{}','{\"issued_on\":\"2025-12-31\",\"tax_on\":\"2026-01-10\",\"calculation\":{\"tax_mode\":\"vat_payer\"}}','{\"currency\":\"CZK\",\"payable\":\"70.25\",\"amounts\":{\"net\":\"60.00\"}}','TEST-1',now(),$4,$2,1,'{\"credited\":\"50.00\",\"rounding_adjustment\":\"-0.75\",\"sources\":[{\"tax_receipts\":[{\"totals\":{\"amounts\":{\"net\":\"40.00\"}}}]}]}') RETURNING id")
 .bind(c_id).bind(a_id).bind(uuid::Uuid::new_v4()).bind(uuid::Uuid::new_v4()).fetch_one(&p).await.unwrap();
    sqlx::query("INSERT INTO credit_note(company_id,invoice_id,number,status,source,data,totals) VALUES($1,$2,'TEST-C','issued','{}','{\"issued_on\":\"2026-02-01\"}','{\"currency\":\"CZK\",\"payable\":\"-21.00\",\"amounts\":{\"net\":\"-17.36\"}}')").bind(c_id).bind(invoice).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO retail_document(company_id,number,data,totals,allocations) VALUES($1,'TEST-R','{\"invoice\":{\"issued_on\":\"2026-02-01\"}}','{\"payable\":\"10.01\",\"amounts\":{\"net\":\"8.27\"}}','[]')").bind(c_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) SELECT $1,'P-'||i,'{\"issued_on\":\"2025-12-31\",\"lines\":[{\"net\":\"0.83\"}]}',1.01,'2026-03-01','2026-03-15' FROM generate_series(1,27) i").bind(c_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on,cancelled) VALUES($1,'CANCEL','{\"issued_on\":\"2025-12-31\"}',999,'2026-03-01','2026-03-15',true)").bind(c_id).execute(&p).await.unwrap();
    let s = PgPeriodReport(p.clone());
    let r = s.report(&a, &c, query(1)).await.unwrap();
    assert_eq!(r.sales, "110.01");
    assert_eq!(r.purchases, "27.27");
    assert_eq!(r.total, 30);
    assert_eq!(r.items.len(), 25);
    assert_eq!(r.months.len(), 12);
    assert_eq!(r.months[0].sales, "121.00");
    assert_eq!(r.months[1].sales, "-10.99");
    assert_eq!(r.months[11].sales, "0.00");
    let mut net = query(1);
    net.value_mode = faktura_application::period_report::ValueMode::Net;
    let nr = s.report(&a, &c, net).await.unwrap();
    assert_eq!(nr.sales, "90.91");
    assert_eq!(nr.purchases, "22.41");
    sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) VALUES($1,'PREVIOUS','{\"issued_on\":\"2025-01-10\",\"lines\":[{\"net\":\"10.00\"}]}',12.10,'2025-01-10','2025-01-15')").bind(c_id).execute(&p).await.unwrap();
    let mut comparison = query(1);
    comparison.cumulative = true;
    comparison.compare_previous = true;
    let compared = s.report(&a, &c, comparison).await.unwrap();
    assert_eq!(compared.months[1].sales, "110.01");
    assert_eq!(compared.months[11].sales, "110.01");
    assert_eq!(compared.months[11].purchases, "27.27");
    let previous = compared.comparison.unwrap();
    assert_eq!(previous.from.to_string(), "2025-01-01");
    assert_eq!(previous.purchases, "12.10");
    assert_eq!(previous.purchases_difference, "15.17");
    assert_eq!(previous.months[11].purchases, "12.10");
    let mut day = query(1);
    day.from = "2026-01-10".parse().unwrap();
    day.to = day.from;
    day.compare_previous = true;
    let day = s.report(&a, &c, day).await.unwrap();
    assert_eq!(day.months.len(), 1);
    assert_eq!(day.sales, "121.00");
    assert_eq!(day.comparison.unwrap().purchases_difference, "-12.10");
    let second = s.report(&a, &c, query(2)).await.unwrap();
    assert_eq!(second.items.len(), 5);
    assert_eq!(second.sales, r.sales);
    assert_eq!(s.report(&a, &other, query(1)).await.unwrap().total, 0);
    assert!(s.report("999999", &c, query(1)).await.is_err());
    let mut invalid = query(1);
    invalid.to = "2027-01-03".parse().unwrap();
    assert!(s.report(&a, &c, invalid).await.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c_id)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.report(&a, &c, query(1)).await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_and_mcp_report_match(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (_, token, c, _) = setup(&p).await;
    let router = faktura_server::period_report::router(
        p.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/reports/period?from=2026-01-01&to=2026-12-31&cumulative=true&compare_previous=true"
                ))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1000000).await.unwrap()).unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(p.clone(), token.clone());
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
    let result = client
        .call_tool(
            CallToolRequestParams::new("period_report").with_arguments(
                serde_json::json!({"company_id":c,"from":"2026-01-01","to":"2026-12-31","cumulative":true,"compare_previous":true})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let json = serde_json::to_value(result).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(json["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http, mcp);
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[test]
fn previous_year_clamps_leap_day_and_validates_boundaries() {
    let mut q = query(1);
    q.from = "2024-02-29".parse().unwrap();
    q.to = q.from;
    q.compare_previous = true;
    assert!(q.validate().is_ok());
    assert_eq!(q.previous_period().0.to_string(), "2023-02-28");
    q.from = "1900-01-01".parse().unwrap();
    q.to = q.from;
    assert!(q.validate().is_err());
    q.compare_previous = false;
    assert!(q.validate().is_ok());
}

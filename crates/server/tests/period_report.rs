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
        grouping: faktura_application::period_report::Grouping::Month,
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
    let mut quarter = query(1);
    quarter.grouping = faktura_application::period_report::Grouping::Quarter;
    quarter.compare_previous = true;
    let qr = s.report(&a, &c, quarter).await.unwrap();
    assert_eq!(qr.months.len(), 4);
    assert_eq!(qr.months[0].month, "2026-Q1");
    assert_eq!(qr.months[0].sales, "110.01");
    assert_eq!(qr.months[0].purchases, "27.27");
    assert_eq!(qr.months[1].sales, "0.00");
    assert_eq!(qr.comparison.unwrap().months[0].month, "2025-Q1");
    let mut year = query(1);
    year.grouping = faktura_application::period_report::Grouping::Year;
    year.from = "2024-01-01".parse().unwrap();
    let yr = s.report(&a, &c, year).await.unwrap();
    assert_eq!(yr.months.len(), 3);
    assert_eq!(yr.months[0].month, "2024");
    assert_eq!(yr.months[0].purchases, "0.00");
    assert_eq!(yr.months[1].purchases, "12.10");
    assert_eq!(yr.months[2].purchases, "27.27");
    let mut partial = query(1);
    partial.grouping = faktura_application::period_report::Grouping::Quarter;
    partial.from = "2026-02-01".parse().unwrap();
    partial.cumulative = true;
    let partial = s.report(&a, &c, partial).await.unwrap();
    assert_eq!(partial.months[0].sales, "-10.99");
    assert_eq!(partial.months[3].sales, "-10.99");
    assert_eq!(partial.months[3].purchases, "27.27");
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
    sqlx::query("WITH a AS (INSERT INTO cash_account(company_id,name,opening_on,opening_balance,currency) VALUES($1,'Parity','2024-01-01',0,'CZK') RETURNING id) INSERT INTO cash_movement(company_id,account_id,number,data,issuer,amount,booked_on,tax) SELECT $1,id,'PARITY','{}','{}',121,current_date,jsonb_build_object('tax',jsonb_build_object('tax_on',current_date::text),'amounts',jsonb_build_object('net','100.00','vat','21.00')) FROM a").bind(c.parse::<i64>().unwrap()).execute(&p).await.unwrap();

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
                    "/api/companies/{c}/reports/period?from=2026-01-01&to=2026-12-31&cumulative=true&compare_previous=true&grouping=quarter"
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
                serde_json::json!({"company_id":c,"from":"2026-01-01","to":"2026-12-31","cumulative":true,"compare_previous":true,"grouping":"quarter"})
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
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/companies/{c}/reports/dashboard?all_years=true"
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
    let result = client
        .call_tool(
            CallToolRequestParams::new("dashboard_report").with_arguments(
                serde_json::json!({"company_id":c,"all_years":true})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let result = serde_json::to_value(result).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(http, mcp);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/reports/tax?quarterly=true"))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let http: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1000000).await.unwrap()).unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("tax_report").with_arguments(
                serde_json::json!({"company_id":c,"quarterly":true})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let result = serde_json::to_value(result).unwrap();
    let mcp: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
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

#[test]
fn grouping_limits_and_unknown_values_are_explicit() {
    use faktura_application::period_report::Grouping;
    for (grouping, last_valid, first_invalid) in [
        (Grouping::Month, "2027-01-02", "2027-01-03"),
        (Grouping::Quarter, "2029-01-01", "2029-01-02"),
        (Grouping::Year, "2036-01-02", "2036-01-03"),
    ] {
        let mut q = query(1);
        q.grouping = grouping;
        q.to = last_valid.parse().unwrap();
        assert!(q.validate().is_ok());
        q.to = first_invalid.parse().unwrap();
        assert!(q.validate().is_err());
    }
    let value = serde_json::json!({"from":"2026-01-01","to":"2026-12-31","grouping":"week"});
    assert!(serde_json::from_value::<Search>(value).is_err());
    let q: Search =
        serde_json::from_value(serde_json::json!({"from":"2026-01-01","to":"2026-12-31"})).unwrap();
    assert!(q.grouping == Grouping::Month);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn quarter_boundaries_and_year_comparison_use_the_same_window(p: PgPool) {
    use faktura_application::period_report::{Grouping, ValueMode};
    let (a, _, c, _) = setup(&p).await;
    for (day, amount, net) in [
        ("2025-03-31", "1.21", "1.00"),
        ("2026-03-31", "2.42", "2.00"),
        ("2026-04-01", "3.63", "3.00"),
        ("2026-12-31", "4.84", "4.00"),
        ("2027-01-01", "6.05", "5.00"),
    ] {
        sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) VALUES($1::text::bigint,$2,$3,$4::text::numeric,$5::text::date,$5::text::date)")
            .bind(&c).bind(day).bind(serde_json::json!({"issued_on":day,"lines":[{"net":net}]}))
            .bind(amount).bind(day).execute(&p).await.unwrap();
    }
    let store = PgPeriodReport(p);
    let mut q = query(1);
    q.grouping = Grouping::Quarter;
    q.compare_previous = true;
    let report = store.report(&a, &c, q).await.unwrap();
    assert_eq!(
        report
            .months
            .iter()
            .map(|m| m.purchases.as_str())
            .collect::<Vec<_>>(),
        vec!["2.42", "3.63", "0.00", "4.84"]
    );
    assert_eq!(report.purchases, "10.89");
    assert_eq!(report.comparison.unwrap().purchases_difference, "9.68");
    let mut q = query(1);
    q.grouping = Grouping::Year;
    q.from = "2025-04-01".parse().unwrap();
    q.to = "2027-01-01".parse().unwrap();
    q.value_mode = ValueMode::Net;
    q.cumulative = true;
    let report = store.report(&a, &c, q).await.unwrap();
    assert_eq!(
        report
            .months
            .iter()
            .map(|m| m.purchases.as_str())
            .collect::<Vec<_>>(),
        vec!["0.00", "9.00", "14.00"]
    );
    assert_eq!(report.purchases, "14.00");
    assert_eq!(report.total, 4);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn dashboard_balances_rankings_aging_reversals_and_scope(p: PgPool) {
    use faktura_application::dashboard::{DashboardStore, Search as DashboardSearch};
    use faktura_infrastructure::dashboard::PgDashboard;
    let (a, _, c, other) = setup(&p).await;
    let cid: i64 = c.parse().unwrap();
    let aid: i64 = a.parse().unwrap();
    let year: i32 = sqlx::query_scalar("SELECT extract(year FROM (current_date-200))::integer")
        .fetch_one(&p)
        .await
        .unwrap();
    let mut invoices = Vec::new();
    for days in [91, 90, 61, 60, 31, 30, 1, 0] {
        let id:i64=sqlx::query_scalar(r#"INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,number,issued_at,issue_request_id,issue_actor_id,issue_revision)
        VALUES($1,$2,$3,'{}',jsonb_build_object('issued_on',(current_date-200)::text,'due_on',(current_date-$4)::text,'contact_id','42','customer',jsonb_build_object('name','Synthetic Alpha'),'calculation',jsonb_build_object('tax_mode','non_payer')),
        '{"currency":"CZK","payable":"100.00","amounts":{"net":"80.00"}}',$5,now(),$6,$2,1) RETURNING id"#)
        .bind(cid).bind(aid).bind(uuid::Uuid::new_v4()).bind(days).bind(format!("DASH-{days}")).bind(uuid::Uuid::new_v4()).fetch_one(&p).await.unwrap();
        invoices.push(id);
    }
    for amount in [40, 10] {
        let id:i64=sqlx::query_scalar("INSERT INTO invoice_payment(company_id,invoice_id,amount,paid_on,method,note,actor_id) VALUES($1,$2,$3,current_date,'cash','synthetic',$4) RETURNING id").bind(cid).bind(invoices[0]).bind(amount).bind(aid).fetch_one(&p).await.unwrap();
        if amount == 10 {
            sqlx::query("INSERT INTO invoice_payment_reversal(company_id,invoice_id,payment_id,reason,actor_id) VALUES($1,$2,$3,'synthetic',$4)").bind(cid).bind(invoices[0]).bind(id).bind(aid).execute(&p).await.unwrap();
        }
    }
    let credit:i64=sqlx::query_scalar(r#"INSERT INTO credit_note(company_id,invoice_id,number,status,source,data,totals) VALUES($1,$2,'DASH-C','issued','{}',jsonb_build_object('issued_on',(current_date-200)::text),'{"currency":"CZK","payable":"-40.00","amounts":{"net":"-32.00"}}') RETURNING id"#).bind(cid).bind(invoices[0]).fetch_one(&p).await.unwrap();
    sqlx::query("INSERT INTO credit_note_event(company_id,credit_id,invoice_id,kind,amount,date,method,reason,actor_id) VALUES($1,$2,$3,'offset',20,current_date,'offset','synthetic',$4)").bind(cid).bind(credit).bind(invoices[0]).bind(aid).execute(&p).await.unwrap();
    sqlx::query(r#"INSERT INTO retail_document(company_id,number,data,totals,allocations) VALUES($1,'DASH-R',jsonb_build_object('invoice',jsonb_build_object('issued_on',(current_date-200)::text)),'{"payable":"10.00","amounts":{"net":"8.00"}}','[]')"#).bind(cid).execute(&p).await.unwrap();
    sqlx::query(r#"INSERT INTO received_invoice(company_id,number,data,total,paid,received_on,due_on) VALUES($1,'DASH-P','{"lines":[{"net":"166.00"}]}',200,50,current_date-200,current_date-31)"#).bind(cid).execute(&p).await.unwrap();
    sqlx::query(r#"INSERT INTO received_invoice(company_id,number,kind,data,total,paid,received_on,due_on) VALUES($1,'DASH-RECEIPT','receipt',jsonb_build_object('issued_on',(current_date-200)::text,'lines',jsonb_build_array(jsonb_build_object('net','40.00'))),50,50,current_date-200,current_date-200)"#).bind(cid).execute(&p).await.unwrap();
    sqlx::query(r#"INSERT INTO sales_document(company_id,kind,creator_id,request_id,initial_data,data,number,advance_totals,advance_paid) VALUES($1,'advance',$2,$3,'{}',jsonb_build_object('invoice',jsonb_build_object('issued_on',(current_date-200)::text,'due_on',(current_date-1)::text,'contact_id','42','customer',jsonb_build_object('name','Synthetic Alpha'))),'DASH-A','{"currency":"CZK","payable":"1000.00"}',100)"#).bind(cid).bind(aid).bind(uuid::Uuid::new_v4()).execute(&p).await.unwrap();
    let store = PgDashboard(p.clone());
    let report = store
        .dashboard(
            &a,
            &c,
            DashboardSearch {
                year: Some(year),
                all_years: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(report.sales.gross, "770.00");
    assert_eq!(report.sales.net, "616.00");
    assert_eq!(report.sales.outstanding, "720.00");
    assert_eq!(report.sales.settled, "50.00");
    assert_eq!(report.purchases.gross, "250.00");
    assert_eq!(report.purchases.net, "206.00");
    assert_eq!(report.purchases.outstanding, "150.00");
    assert_eq!(report.purchases.settled, "100.00");
    let sales: Vec<_> = report
        .aging
        .iter()
        .filter(|v| v.side == "sales")
        .map(|v| (v.bucket.as_str(), v.amount.as_str(), v.count))
        .collect();
    assert_eq!(
        sales,
        vec![
            ("not_due", "100.00", 1),
            ("days_1_30", "200.00", 2),
            ("days_31_60", "200.00", 2),
            ("days_61_90", "200.00", 2),
            ("days_91_plus", "40.00", 1)
        ]
    );
    assert_eq!(report.customers[0].amount, "1800.00");
    assert_eq!(report.debtors[0].amount, "1540.00");
    assert_eq!(report.customers[0].count, 9);
    let empty = store
        .dashboard(&a, &other, DashboardSearch::default())
        .await
        .unwrap();
    assert_eq!(empty.sales.gross, "0.00");
    assert!(empty.debtors.is_empty());
    assert!(store
        .dashboard("999999", &c, DashboardSearch::default())
        .await
        .is_err());
    assert!(store
        .dashboard(
            &a,
            &c,
            DashboardSearch {
                year: Some(9999),
                all_years: true
            }
        )
        .await
        .is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(cid)
        .execute(&p)
        .await
        .unwrap();
    assert!(store
        .dashboard(&a, &c, DashboardSearch::default())
        .await
        .is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_report_stored_vat_advance_deductions_and_turnover(p: PgPool) {
    use faktura_application::tax_report::{Search as Q, TaxReportStore};
    use faktura_infrastructure::tax_report::PgTaxReport;
    let (a, _, c, other) = setup(&p).await;
    let cid: i64 = c.parse().unwrap();
    let aid: i64 = a.parse().unwrap();
    let invoice:i64=sqlx::query_scalar("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,number,issued_at,issue_request_id,issue_actor_id,issue_revision,advances) VALUES($1,$2,$3,'{}','{\"issued_on\":\"2024-12-31\",\"tax_on\":\"2025-02-01\",\"calculation\":{\"tax_mode\":\"vat_payer\"}}','{\"currency\":\"CZK\",\"payable\":\"60.50\",\"amounts\":{\"net\":\"50.00\",\"vat\":\"10.50\"}}','TAX-1',now(),$4,$2,1,'{\"credited\":\"60.50\",\"sources\":[{\"tax_receipts\":[{\"totals\":{\"amounts\":{\"net\":\"50.00\",\"vat\":\"10.50\"}}}]}]}') RETURNING id")
 .bind(cid).bind(aid).bind(uuid::Uuid::new_v4()).bind(uuid::Uuid::new_v4()).fetch_one(&p).await.unwrap();
    let advance:i64=sqlx::query_scalar("INSERT INTO sales_document(company_id,kind,creator_id,request_id,initial_data,data) VALUES($1,'advance',$2,$3,'{}','{}') RETURNING id").bind(cid).bind(aid).bind(uuid::Uuid::new_v4()).fetch_one(&p).await.unwrap();
    let payment:i64=sqlx::query_scalar("INSERT INTO advance_payment(company_id,document_id,actor_id,request_id,command,amount,paid_on,method,note) VALUES($1,$2,$3,$4,'{}',60.50,'2025-01-01','bank_transfer','') RETURNING id").bind(cid).bind(advance).bind(aid).bind(uuid::Uuid::new_v4()).fetch_one(&p).await.unwrap();
    sqlx::query("INSERT INTO tax_receipt(company_id,advance_id,payment_id,actor_id,request_id,command,number,snapshot,pdf) VALUES($1,$2,$3,$4,$5,'{}','DDPP','{\"data\":{\"issued_on\":\"2025-01-02\"},\"totals\":{\"currency\":\"CZK\",\"amounts\":{\"vat\":\"10.50\"}}}','')").bind(cid).bind(advance).bind(payment).bind(aid).bind(uuid::Uuid::new_v4()).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO credit_note(company_id,invoice_id,number,status,source,data,totals) VALUES($1,$2,'TAX-C','issued','{\"data\":{\"calculation\":{\"tax_mode\":\"vat_payer\"}}}','{\"issued_on\":\"2025-03-01\",\"tax_on\":\"2025-04-01\"}','{\"currency\":\"CZK\",\"payable\":\"-12.10\",\"amounts\":{\"net\":\"-10.00\",\"vat\":\"-2.10\"}}')").bind(cid).bind(invoice).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) SELECT $1,'P-'||i,'{\"currency\":\"CZK\",\"issued_on\":\"2024-12-31\",\"tax_on\":\"2025-01-15\",\"lines\":[{\"net\":\"1.00\",\"vat\":\"0.21\"}]}',1.21,'2025-02-01','2025-02-15' FROM generate_series(1,27) i").bind(cid).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on,cancelled) VALUES($1,'CANCEL','{\"currency\":\"CZK\",\"tax_on\":\"2025-01-15\",\"lines\":[{\"vat\":\"999\"}]}',999,'2025-01-01','2025-01-01',true)").bind(cid).execute(&p).await.unwrap();
    let q = || Q {
        year: Some(2025),
        ..Default::default()
    };
    let store = PgTaxReport(p.clone());
    let r = store.report(&a, &c, q()).await.unwrap();
    assert_eq!(r.output, "18.90");
    assert_eq!(r.input, "5.67");
    assert_eq!(r.difference, "13.23");
    assert_eq!(r.turnover, "90.00");
    assert_eq!(r.total, 30);
    assert_eq!(r.items.len(), 25);
    assert_eq!(r.periods.len(), 12);
    assert_eq!(r.periods[0].output, "10.50");
    assert_eq!(r.periods[1].output, "10.50");
    assert_eq!(r.periods[3].output, "-2.10");
    let r = store
        .report(
            &a,
            &c,
            Q {
                quarterly: true,
                page: Some(2),
                ..q()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.periods.len(), 4);
    assert_eq!(r.periods[0].output, "21.00");
    assert_eq!(r.items.len(), 5);
    assert_eq!(r.input, "5.67");
    let r = store
        .report(
            &a,
            &c,
            Q {
                turnover_sources: true,
                ..q()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.total, 2);
    assert_eq!(r.items[0].amount, "-10.00");
    assert_eq!(store.report(&a, &other, q()).await.unwrap().output, "0.00");
    assert!(store.report("999999", &c, q()).await.is_err());
    assert!(store
        .report(
            &a,
            &c,
            Q {
                year: Some(1899),
                ..q()
            }
        )
        .await
        .is_err());
    assert!(store
        .report(
            &a,
            &c,
            Q {
                page: Some(0),
                ..q()
            }
        )
        .await
        .is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(cid)
        .execute(&p)
        .await
        .unwrap();
    assert!(store.report(&a, &c, q()).await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn standalone_cash_tax_and_turnover_use_snapshots_without_double_counting(p: PgPool) {
    use faktura_application::tax_report::{Search as Q, TaxReportStore};
    use faktura_infrastructure::tax_report::PgTaxReport;
    let (a, _, c, other) = setup(&p).await;
    let cid: i64 = c.parse().unwrap();
    let account:i64=sqlx::query_scalar("INSERT INTO cash_account(company_id,name,opening_on,opening_balance,currency) VALUES($1,'Synthetic','2024-01-01',0,'CZK') RETURNING id").bind(cid).fetch_one(&p).await.unwrap();
    // Gross ledger amounts, immutable positive tax breakdown, different booking/DUZP years.
    for (number, amount, net, vat, tax_on, cancelled) in [
        ("IN", 121.0, "100.00", "21.00", "2025-12-31", false),
        ("OUT", -112.0, "100.00", "12.00", "2025-01-01", false),
        ("ZERO", 5.02, "5.02", "0.00", "2025-06-01", false),
        ("CENT", 0.03, "0.02", "0.01", "2025-06-01", false),
        ("CANCEL", 1210.0, "1000.00", "210.00", "2025-06-01", true),
        ("OLD", 1210.0, "1000.00", "210.00", "2024-12-31", false),
    ] {
        let tax = serde_json::json!({"input_amount":"ignored","tax":{"tax_on":tax_on},"amounts":{"net":net,"vat":vat}});
        sqlx::query("INSERT INTO cash_movement(company_id,account_id,number,data,issuer,amount,booked_on,tax,cancelled) VALUES($1,$2,$3,'{}','{}',$4::text::numeric,'2026-01-02',$5,$6)").bind(cid).bind(account).bind(number).bind(amount.to_string()).bind(tax).bind(cancelled).execute(&p).await.unwrap();
    }
    // Old/manual and automatic cash payments contain no separate taxable supply.
    for automatic in [false, true] {
        sqlx::query("INSERT INTO cash_movement(company_id,account_id,number,data,issuer,amount,booked_on,automatic) VALUES($1,$2,$3,'{}','{}',99999,'2025-01-01',$4)").bind(cid).bind(account).bind(format!("PAY-{automatic}")).bind(automatic).execute(&p).await.unwrap();
    }
    // Archive and today's non-payer status must not erase historical evidence.
    sqlx::query("UPDATE cash_account SET archived=true WHERE id=$1")
        .bind(account)
        .execute(&p)
        .await
        .unwrap();
    let store = PgTaxReport(p.clone());
    let q = || Q {
        year: Some(2025),
        ..Default::default()
    };
    let r = store.report(&a, &c, q()).await.unwrap();
    assert_eq!(r.tax_mode, "non_payer");
    assert_eq!(r.output, "21.01");
    assert_eq!(r.input, "12.00");
    assert_eq!(r.difference, "9.01");
    assert_eq!(r.turnover, "105.04");
    assert_eq!(r.total, 4);
    assert_eq!(r.periods[0].input, "12.00");
    assert_eq!(r.periods[11].output, "21.00");
    assert!(r.items.iter().all(|i| i.kind == "cash"));
    let r = store
        .report(
            &a,
            &c,
            Q {
                quarterly: true,
                turnover_sources: true,
                ..q()
            },
        )
        .await
        .unwrap();
    assert_eq!(r.total, 3);
    assert_eq!(r.periods[3].output, "21.00");
    assert!(r.items.iter().all(|i| i.side == "sales"));
    sqlx::query("UPDATE cash_movement SET cancelled=true,reason='Synthetic reversal' WHERE company_id=$1 AND number='IN'").bind(cid).execute(&p).await.unwrap();
    let r = store.report(&a, &c, q()).await.unwrap();
    assert_eq!(r.output, "0.01");
    assert_eq!(r.turnover, "5.04");
    assert_eq!(r.difference, "-11.99");
    let r = store.report(&a, &other, q()).await.unwrap();
    assert_eq!(r.total, 0);
    assert_eq!(r.turnover, "0.00");
    assert!(store.report("999999", &c, q()).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn foreign_invoice_issue_payment_and_report_snapshot(p: PgPool) {
    use faktura_application::dashboard::DashboardStore;
    use faktura_application::{draft, payment};
    use faktura_infrastructure::{dashboard::PgDashboard, draft::PgDraft, payment::PgPayment};
    let (a, _, c, other) = setup(&p).await;
    let today: sqlx::types::chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&p)
            .await
            .unwrap();
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.issued_on = Some(today);
    data.due_on = Some(today);
    data.tax_on = None;
    data.calculation.currency = "EUR".into();
    data.calculation.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "25.00".into(),
        units: "1".into(),
    });
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    data.calculation.rounding = faktura_domain::invoice::Rounding::None;
    data.calculation.discount_percent = "0".into();
    data.calculation.lines.truncate(1);
    let line = &mut data.calculation.lines[0];
    line.quantity = "1".into();
    line.unit_price = "100".into();
    line.discount_percent = "0".into();
    line.vat_percent = "0".into();
    data.supplier.registration_number = "12345678".into();
    data.payment.method = "bank_transfer".into();
    data.payment.bank_account = "12345678/0100".into();
    let store = PgDraft(p.clone());
    let d = draft::create(
        &store,
        &a,
        &c,
        draft::Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            data,
        },
    )
    .await
    .unwrap();
    let d = draft::issue(
        &store,
        &a,
        &c,
        &d.id,
        draft::Issue {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    assert_eq!(d.totals.as_ref().unwrap().currency, "EUR");
    assert_eq!(d.totals.as_ref().unwrap().payable, "100.00");
    let initial = payment::get(&PgPayment(p.clone()), &a, &c, &d.id)
        .await
        .unwrap();
    let paid = payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: initial.summary.revision,
            payment: faktura_domain::payment::PaymentInput {
                amount: "40.00".into(),
                paid_on: today,
                method: "bank_transfer".into(),
                note: String::new(),
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(paid.summary.outstanding, "60.00");
    let r = PgDashboard(p.clone())
        .dashboard(&a, &c, Default::default())
        .await
        .unwrap();
    assert_eq!(r.sales.gross, "2500.00");
    assert_eq!(r.sales.outstanding, "1500.00");
    assert_eq!(r.sales.settled, "1000.00");
    assert_eq!(r.customers[0].amount, "2500.00");
    use faktura_application::payment::PaymentStore;
    let r = PgPayment(p.clone()).overview(&a, &c).await.unwrap();
    assert_eq!(r.outstanding, "1500.00");
    assert_eq!(r.received, "1000.00");
    assert_eq!(
        PgDashboard(p.clone())
            .dashboard(&a, &other, Default::default())
            .await
            .unwrap()
            .sales
            .gross,
        "0.00"
    );
    let value: rust_decimal::Decimal =
        sqlx::query_scalar("SELECT report_czk(-0.01,'EUR','{\"rate\":\"25.5\",\"units\":\"1\"}')")
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(value.to_string(), "-0.26");
    assert!(sqlx::query("SELECT report_czk(100,'EUR',NULL)")
        .execute(&p)
        .await
        .is_err());
    let bad = payment::register(
        &PgPayment(p.clone()),
        &a,
        &c,
        &d.id,
        payment::Register {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: paid.summary.revision,
            payment: faktura_domain::payment::PaymentInput {
                amount: "1".into(),
                paid_on: today,
                method: "cash".into(),
                note: String::new(),
            },
        },
    )
    .await;
    assert!(bad.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn tax_evidence_exports_all_sources_from_one_snapshot(p: PgPool) {
    use faktura_application::{
        export::{Format, Language},
        tax_report::{ExportRequest, Search as TaxSearch, TaxExportStore},
    };
    let (a, _, c, other) = setup(&p).await;
    sqlx::query("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) SELECT $1,'=SYNTHETIC-'||i,'{\"issued_on\":\"2026-01-01\",\"tax_on\":\"2026-01-01\",\"lines\":[{\"net\":\"100.01\",\"vat\":\"21.02\"}]}',121.03,'2026-01-01','2026-01-15' FROM generate_series(1,30) i").bind(c.parse::<i64>().unwrap()).execute(&p).await.unwrap();
    let request = |format| ExportRequest {
        search: TaxSearch {
            year: Some(2026),
            page: Some(9999),
            ..Default::default()
        },
        format,
        language: Language::En,
    };
    let store = faktura_infrastructure::tax_report::PgTaxReport(p.clone());
    let f = store.export(&a, &c, request(Format::Csv)).await.unwrap();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(&f.bytes[3..]);
    let rows: Vec<_> = reader.records().map(Result::unwrap).collect();
    assert_eq!(rows.len(), 30);
    assert!(rows[0][2].starts_with("'=SYNTHETIC-"));
    assert_eq!(&rows[0][5], "21.02");
    assert!(rows[0][6].contains("not a tax return"));
    let f = store.export(&a, &c, request(Format::Xlsx)).await.unwrap();
    assert!(f.bytes.starts_with(b"PK"));
    let f = store
        .export(&a, &other, request(Format::Csv))
        .await
        .unwrap();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(&f.bytes[3..]);
    assert_eq!(reader.records().count(), 0);
    assert!(store
        .export(&a, "999999", request(Format::Csv))
        .await
        .is_err());
}

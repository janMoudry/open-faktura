#[path = "support/historical_documents.rs"]
mod historical_documents;
use faktura_application::{
    auth, company,
    sales::{self, Create, Data, Error, Kind, Update},
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

use faktura_application::delivery::{self, SendQuote};
use faktura_infrastructure::delivery::{MailConfig, PgDelivery};
use std::sync::Arc;
fn command(revision: i64, key: &str) -> SendQuote {
    SendQuote {
        purpose: None,
        settlement_revision: None,
        automation_revision: None,
        attachment_ids: vec![],
        share_id: None,
        request_id: key.into(),
        revision,
        recipient: "customer@example.test".into(),
        subject: "Žluťoučká nabídka – zkouška".into(),
        body: "Dobrý den,\n\nv příloze nabídka.\nDěkujeme.".into(),
    }
}
async fn smtp(reject: bool) -> (u16, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let (reader, mut writer) = socket.into_split();
        let mut reader = BufReader::new(reader);
        writer.write_all(b"220 capture ESMTP\r\n").await.unwrap();
        let mut message = String::new();
        let mut data = false;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            if data {
                if line == ".\r\n" {
                    writer.write_all(b"250 accepted\r\n").await.unwrap();
                    break;
                } else {
                    message.push_str(&line)
                }
                continue;
            }
            let response: &[u8] = if line.starts_with("EHLO") {
                b"250 capture\r\n".as_slice()
            } else if line.starts_with("DATA") {
                data = true;
                b"354 proceed\r\n"
            } else if reject && line.starts_with("RCPT") {
                b"550 rejected\r\n"
            } else {
                b"250 ok\r\n"
            };
            writer.write_all(response).await.unwrap();
            if reject && line.starts_with("RCPT") {
                break;
            }
        }
        message
    });
    (port, task)
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and local SMTP capture"]
async fn immutable_pdf_queue_replay_smtp_and_scope(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let sales = PgSales(pool.clone());
    let q = sales::create(&sales, &a, &c, create(Kind::Quote, KEY))
        .await
        .unwrap();
    let (port, capture) = smtp(false).await;
    let store = PgDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let cmd = command(1, KEY);
    let (x, y) = tokio::join!(
        delivery::send(&store, &a, &c, &q.id, cmd.clone()),
        delivery::send(&store, &a, &c, &q.id, cmd.clone())
    );
    let sent = x.unwrap();
    assert_eq!(sent.id, y.unwrap().id);
    assert_eq!(sent.status, "queued");
    let bytes = delivery::pdf(&store, &a, &c, &q.id, Some(&sent.id))
        .await
        .unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    assert!(matches!(
        delivery::pdf(&store, &a, &other, &q.id, Some(&sent.id)).await,
        Err(Error::NotFound)
    ));
    assert!(store.process_one().await.unwrap());
    assert!(!store.process_one().await.unwrap());
    let mime = tokio::time::timeout(std::time::Duration::from_secs(3), capture)
        .await
        .unwrap()
        .unwrap();
    assert!(mime.contains("Content-Type: application/pdf"));
    assert!(mime.contains("CN20260001.pdf"));
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    assert!(mime.replace("\r\n", "").contains(&encoded));
    let h = delivery::history(&store, &a, &c, &q.id, 1).await.unwrap();
    assert_eq!(h.total, 1);
    assert_eq!(h.items[0].status, "sent");
    assert!(h.items[0].finished_at.is_some());
    let mut now = sales::get(&sales, &a, &c, &q.id).await.unwrap();
    assert_eq!(now.data.status, "sent");
    assert_eq!(now.revision, 2);
    assert_eq!(
        delivery::send(&store, &a, &c, &q.id, cmd.clone())
            .await
            .unwrap()
            .id,
        sent.id
    );
    let mut wrong = cmd;
    wrong.subject = "different".into();
    assert!(matches!(
        delivery::send(&store, &a, &c, &q.id, wrong).await,
        Err(Error::Conflict)
    ));
    now.data.name = "Later version".into();
    sales::update(
        &sales,
        &a,
        &c,
        &q.id,
        Update {
            revision: 2,
            archived: false,
            data: now.data,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        bytes,
        delivery::pdf(&store, &a, &c, &q.id, Some(&sent.id))
            .await
            .unwrap()
    );
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('reader@example.test','no-login') RETURNING id").fetch_one(&pool).await.unwrap();
    let viewer = viewer.to_string();
    assert!(matches!(
        delivery::history(&store, &viewer, &c, &q.id, 1).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(c.parse::<i64>().unwrap())
        .bind(viewer.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(delivery::pdf(&store, &viewer, &c, &q.id, Some(&sent.id))
        .await
        .is_ok());
    assert!(matches!(
        delivery::send(&store, &viewer, &c, &q.id, command(3, KEY)).await,
        Err(Error::Forbidden)
    ));
    std::fs::create_dir_all("../../output/pdf").unwrap();
    std::fs::write("../../output/pdf/quotation-cs.pdf", bytes).unwrap();
    let mut d = data();
    d.invoice.language = "en".into();
    let line = d.invoice.calculation.lines[0].clone();
    d.invoice.calculation.lines=(0..60).map(|i|{let mut l=line.clone();l.id=format!("line-{i}");l.description=format!("Consultation {i}: Czech text Žluťoučký kůň, long description for pagination verification.");l}).collect();
    let long = sales::create(
        &sales,
        &a,
        &c,
        Create {
            request_id: "00000000-0000-4000-8000-000000000003".into(),
            kind: Kind::Quote,
            data: d,
        },
    )
    .await
    .unwrap();
    let pdf = delivery::pdf(&store, &a, &c, &long.id, None).await.unwrap();
    std::fs::write("../../output/pdf/quotation-en-multipage.pdf", pdf).unwrap();
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and local SMTP capture"]
async fn rejected_and_interrupted_attempts_are_not_retried(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let q = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Quote, KEY))
        .await
        .unwrap();
    let (port, capture) = smtp(true).await;
    let store = PgDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    delivery::send(&store, &a, &c, &q.id, command(1, KEY))
        .await
        .unwrap();
    assert!(store.process_one().await.unwrap());
    capture.await.unwrap();
    assert_eq!(
        delivery::history(&store, &a, &c, &q.id, 1)
            .await
            .unwrap()
            .items[0]
            .status,
        "failed"
    );
    assert_eq!(
        sales::get(&PgSales(pool.clone()), &a, &c, &q.id)
            .await
            .unwrap()
            .data
            .status,
        "created"
    );
    sqlx::query("UPDATE quote_delivery SET status='sending',started_at=now()-interval '3 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!store.process_one().await.unwrap());
    assert_eq!(
        delivery::history(&store, &a, &c, &q.id, 1)
            .await
            .unwrap()
            .items[0]
            .status,
        "uncertain"
    );
    let disabled = PgDelivery {
        pool: pool.clone(),
        mail: None,
    };
    assert!(
        !delivery::history(&disabled, &a, &c, &q.id, 1)
            .await
            .unwrap()
            .configured
    );
    assert!(matches!(
        delivery::send(
            &disabled,
            &a,
            &c,
            &q.id,
            command(1, "00000000-0000-4000-8000-000000000004")
        )
        .await,
        Err(Error::Invalid(_))
    ));
    let mut invalid = command(1, KEY);
    invalid.recipient = "not-an-email".into();
    assert!(matches!(
        delivery::send(&store, &a, &c, &q.id, invalid).await,
        Err(Error::Invalid(_))
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_pdf_send_history_and_csrf(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (a, token, c, other) = setup(&pool).await;
    let q = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Quote, KEY))
        .await
        .unwrap();
    let store = PgDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(9))),
    };
    let app = faktura_server::delivery::router(
        pool.clone(),
        vec!["https://app.example.test".into()],
        false,
        store.clone(),
    );
    let path = format!("/api/companies/{c}/sales/{}/deliveries", q.id);
    for (cookie, origin, expected) in [
        ("", "https://app.example.test", StatusCode::UNAUTHORIZED),
        (
            token.as_str(),
            "https://evil.example.test",
            StatusCode::UNAUTHORIZED,
        ),
        (token.as_str(), "https://app.example.test", StatusCode::OK),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .method("POST")
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&command(1, KEY)).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
        assert_eq!(r.headers()["cache-control"], "no-store");
    }
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token.clone())
        .with_delivery(store.clone());
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
    let mut args = serde_json::to_value(command(1, KEY)).unwrap();
    args["company_id"] = c.clone().into();
    args["id"] = q.id.clone().into();
    let response = client
        .call_tool(
            CallToolRequestParams::new("sales_send")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(false));
    let mcp: serde_json::Value =
        serde_json::from_str(&response.content[0].as_text().unwrap().text).unwrap();
    let h = delivery::history(&store, &a, &c, &q.id, 1).await.unwrap();
    assert_eq!(h.total, 1);
    assert_eq!(mcp["id"], h.items[0].id);
    let response = client
        .call_tool(
            CallToolRequestParams::new("sales_history").with_arguments(
                serde_json::json!({"company_id":other,"id":q.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(true));
    let response = client
        .call_tool(
            CallToolRequestParams::new("sales_deliverypdf").with_arguments(
                serde_json::json!({"company_id":c,"id":q.id,"delivery":h.items[0].id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(false));
    let mcp: serde_json::Value =
        serde_json::from_str(&response.content[0].as_text().unwrap().text).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("{path}/{}/pdf", h.items[0].id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "application/pdf");
    use base64::Engine;
    assert_eq!(
        to_bytes(response.into_body(), 5_000_000)
            .await
            .unwrap()
            .to_vec(),
        base64::engine::general_purpose::STANDARD
            .decode(mcp["base64"].as_str().unwrap())
            .unwrap()
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade_preserves_quotes(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|m| m.version < 202610110013)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let sales = PgSales(pool.clone());
    let q = historical_documents::sales(&pool, &a, &c, create(Kind::Quote, KEY)).await;
    all.run(&pool).await.unwrap();
    assert_eq!(
        serde_json::to_value(&q).unwrap(),
        serde_json::to_value(sales::get(&sales, &a, &c, &q.id).await.unwrap()).unwrap()
    );
}

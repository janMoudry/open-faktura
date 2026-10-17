use faktura_application::{
    auth, company,
    draft::{self, Create, Error, Issue},
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
use faktura_application::{
    delivery,
    invoice_delivery::{self, SendInvoice},
};
use faktura_infrastructure::{
    delivery::{MailConfig, PgDelivery},
    invoice_delivery::PgInvoiceDelivery,
};
use std::sync::Arc;
fn command(revision: i64, key: &str) -> SendInvoice {
    SendInvoice {
        share_id: None,
        purpose: None,
        settlement_revision: None,
        attachment_ids: vec![],
        request_id: key.into(),
        revision,
        recipient: "customer@example.test".into(),
        subject: "Faktura – Žluťoučký kůň".into(),
        body: "Dobrý den,\n\nv příloze je faktura.".into(),
    }
}
async fn issued(pool: &PgPool, a: &str, c: &str) -> draft::Draft {
    let store = PgDraft(pool.clone());
    let d = draft::create(&store, a, c, create(KEY)).await.unwrap();
    draft::issue(
        &store,
        a,
        c,
        &d.id,
        Issue {
            request_id: KEY.into(),
            revision: d.revision,
        },
    )
    .await
    .unwrap()
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
async fn issued_pdf_replay_scope_and_unchanged_invoice(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c).await;
    let before = serde_json::to_value(
        draft::get(&PgDraft(pool.clone()), &a, &c, &invoice.id)
            .await
            .unwrap(),
    )
    .unwrap();
    let original = draft::pdf(&PgDraft(pool.clone()), &a, &c, &invoice.id)
        .await
        .unwrap();
    let (port, capture) = smtp(false).await;
    let store = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let cmd = command(invoice.revision, KEY);
    let (x, y) = tokio::join!(
        invoice_delivery::send(&store, &a, &c, &invoice.id, cmd.clone()),
        invoice_delivery::send(&store, &a, &c, &invoice.id, cmd.clone())
    );
    let sent = x.unwrap();
    assert_eq!(sent.id, y.unwrap().id);
    assert_eq!(
        delivery::pdf(&store, &a, &c, &invoice.id, Some(&sent.id))
            .await
            .unwrap(),
        original
    );
    assert!(matches!(
        invoice_delivery::send(&store, &a, &other, &invoice.id, cmd.clone()).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        delivery::pdf(&store, &a, &other, &invoice.id, Some(&sent.id)).await,
        Err(Error::NotFound)
    ));
    let (x, y) = tokio::join!(store.process_one(), store.process_one());
    assert_eq!(usize::from(x.unwrap()) + usize::from(y.unwrap()), 1);
    let mime = capture.await.unwrap();
    assert!(mime.contains("application/pdf"));
    assert!(mime.contains("FV2026000001.pdf"));
    assert!(mime.contains("Reply-To: reply@example.test"));
    use base64::Engine;
    assert!(mime
        .replace("\r\n", "")
        .contains(&base64::engine::general_purpose::STANDARD.encode(&original)));
    let h = delivery::history(&store, &a, &c, &invoice.id, 1)
        .await
        .unwrap();
    assert_eq!(h.total, 1);
    assert_eq!(h.items[0].status, "sent");
    assert_eq!(
        serde_json::to_value(
            draft::get(&PgDraft(pool.clone()), &a, &c, &invoice.id)
                .await
                .unwrap()
        )
        .unwrap(),
        before
    );
    let unconfigured = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: None,
    };
    assert_eq!(
        invoice_delivery::send(&unconfigured, &a, &c, &invoice.id, cmd.clone())
            .await
            .unwrap()
            .id,
        sent.id
    );
    let mut changed = cmd.clone();
    changed.body = "Changed".into();
    assert!(matches!(
        invoice_delivery::send(&store, &a, &c, &invoice.id, changed).await,
        Err(Error::Conflict)
    ));
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('viewer@example.test','no-login') RETURNING id").fetch_one(&pool).await.unwrap();
    assert!(matches!(
        delivery::history(&store, &viewer.to_string(), &c, &invoice.id, 1).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(c.parse::<i64>().unwrap())
        .bind(viewer)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        delivery::history(&store, &viewer.to_string(), &c, &invoice.id, 1)
            .await
            .is_ok()
    );
    assert!(
        delivery::pdf(&store, &viewer.to_string(), &c, &invoice.id, Some(&sent.id))
            .await
            .is_ok()
    );
    assert!(matches!(
        invoice_delivery::send(&store, &viewer.to_string(), &c, &invoice.id, cmd).await,
        Err(Error::Forbidden)
    ));
    // Failed or missing SMTP never makes a new issued invoice or payment.
    assert!(matches!(
        invoice_delivery::send(
            &unconfigured,
            &a,
            &c,
            &invoice.id,
            command(invoice.revision, "00000000-0000-4000-8000-000000000002")
        )
        .await,
        Err(Error::Invalid(_))
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and local SMTP capture"]
async fn rejection_recovery_and_draft_guards(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let d = draft::create(&PgDraft(pool.clone()), &a, &c, create(KEY))
        .await
        .unwrap();
    let (port, capture) = smtp(true).await;
    let store = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    assert!(matches!(
        invoice_delivery::send(&store, &a, &c, &d.id, command(1, KEY)).await,
        Err(Error::Conflict)
    ));
    let invoice = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &d.id,
        Issue {
            request_id: KEY.into(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    let mut invalid = command(invoice.revision, KEY);
    invalid.recipient = "a@example.test\r\nBcc: attacker@example.test".into();
    assert!(matches!(
        invoice_delivery::send(&store, &a, &c, &d.id, invalid).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        invoice_delivery::send(&store, &a, &c, &d.id, command(999, KEY)).await,
        Err(Error::Conflict)
    ));
    invoice_delivery::send(&store, &a, &c, &d.id, command(invoice.revision, KEY))
        .await
        .unwrap();
    assert!(store.process_one().await.unwrap());
    capture.await.unwrap();
    assert_eq!(
        delivery::history(&store, &a, &c, &d.id, 1)
            .await
            .unwrap()
            .items[0]
            .status,
        "failed"
    );
    sqlx::query(
        "UPDATE invoice_delivery SET status='sending',started_at=now()-interval '3 minutes'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(!store.process_one().await.unwrap());
    assert_eq!(
        delivery::history(&store, &a, &c, &d.id, 1)
            .await
            .unwrap()
            .items[0]
            .status,
        "uncertain"
    );
    // A genuinely new request is explicit resend. A deleted profile cancels it before SMTP.
    invoice_delivery::send(
        &store,
        &a,
        &c,
        &d.id,
        command(invoice.revision, "00000000-0000-4000-8000-000000000002"),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE company SET deleted_at=now()")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!store.process_one().await.unwrap());
    let status: String =
        sqlx::query_scalar("SELECT status FROM invoice_delivery ORDER BY id DESC LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "failed");
    assert!(matches!(
        delivery::history(&store, &a, &c, &d.id, 1).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_csrf_and_pdf_parity(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (a, token, c, other) = setup(&pool).await;
    let d = issued(&pool, &a, &c).await;
    let mail = Some(Arc::new(MailConfig::local_capture(9)));
    let store = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: mail.clone(),
    };
    let app = faktura_server::invoice_delivery::router(
        pool.clone(),
        vec!["https://app.example.test".into()],
        false,
        store.clone(),
    );
    let path = format!("/api/companies/{c}/invoices/{}/deliveries", d.id);
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
                    .body(Body::from(
                        serde_json::to_vec(&command(d.revision, KEY)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
        assert_eq!(r.headers()["cache-control"], "no-store");
    }
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token.clone()).with_delivery(
        PgDelivery {
            pool: pool.clone(),
            mail,
        },
    );
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
    let mut args = serde_json::to_value(command(d.revision, KEY)).unwrap();
    args["company_id"] = c.clone().into();
    args["id"] = d.id.clone().into();
    let response = client
        .call_tool(
            CallToolRequestParams::new("invoice_send")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(false));
    let h = delivery::history(&store, &a, &c, &d.id, 1).await.unwrap();
    assert_eq!(h.total, 1);
    let response = client
        .call_tool(
            CallToolRequestParams::new("invoice_history").with_arguments(
                serde_json::json!({"company_id":other,"id":d.id})
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
            CallToolRequestParams::new("invoice_deliverypdf").with_arguments(
                serde_json::json!({"company_id":c,"id":d.id,"delivery":h.items[0].id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(false));
    let pdf: serde_json::Value =
        serde_json::from_str(&response.content[0].as_text().unwrap().text).unwrap();
    let r = app
        .oneshot(
            Request::builder()
                .uri(format!("{path}/{}/pdf", h.items[0].id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    use base64::Engine;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .encode(to_bytes(r.into_body(), 20000000).await.unwrap()),
        pdf["base64"].as_str().unwrap()
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_preserves_invoice_and_pdf(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|m| m.version < 202610110015)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let d = issued(&pool, &a, &c).await;
    let bytes = draft::pdf(&PgDraft(pool.clone()), &a, &c, &d.id)
        .await
        .unwrap();
    all.run(&pool).await.unwrap();
    // The upgrade appends a view column. Production restarts its pool after migration;
    // discard prepared SELECT * plans here as well before reading the upgraded view.
    let mut connections = Vec::new();
    for _ in 0..pool.size() {
        let mut connection = pool.acquire().await.unwrap();
        sqlx::Connection::clear_cached_statements(&mut *connection)
            .await
            .unwrap();
        connections.push(connection);
    }
    drop(connections);
    assert_eq!(
        draft::pdf(&PgDraft(pool.clone()), &a, &c, &d.id)
            .await
            .unwrap(),
        bytes
    );
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: None,
    };
    assert_eq!(
        delivery::history(&mail, &a, &c, &d.id, 1)
            .await
            .unwrap()
            .total,
        0
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn ui_smtp_settings_drive_invoice_worker_without_restart(pool: PgPool) {
    use faktura_application::smtp::{self, Save, Settings};
    let (a, _, c, _) = setup(&pool).await;
    let d = issued(&pool, &a, &c).await;
    let (port, capture) = smtp(false).await;
    let settings = faktura_infrastructure::smtp::PgSmtp {
        pool: pool.clone(),
        fallback: None,
    };
    let mut cfg = Settings {
        revision: 0,
        mode: "custom".into(),
        host: "127.0.0.1".into(),
        port,
        security: "plain".into(),
        sender: "custom@example.test".into(),
        username: "".into(),
    };
    smtp::save(
        &settings,
        &a,
        Save {
            settings: cfg.clone(),
            password: None,
        },
    )
    .await
    .unwrap();
    let store = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: None,
    };
    let sent = invoice_delivery::send(&store, &a, &c, &d.id, command(d.revision, KEY))
        .await
        .unwrap();
    assert_eq!(sent.sender, "custom@example.test");
    cfg.revision = 1;
    cfg.mode = "disabled".into();
    smtp::save(
        &settings,
        &a,
        Save {
            settings: cfg.clone(),
            password: None,
        },
    )
    .await
    .unwrap();
    assert!(!store.process_one().await.unwrap());
    assert!(
        !delivery::history(&store, &a, &c, &d.id, 1)
            .await
            .unwrap()
            .configured
    );
    cfg.revision = 2;
    cfg.mode = "custom".into();
    smtp::save(
        &settings,
        &a,
        Save {
            settings: cfg,
            password: None,
        },
    )
    .await
    .unwrap();
    assert!(store.process_one().await.unwrap());
    let message = capture.await.unwrap();
    assert!(message.contains("custom@example.test"));
    assert!(message.contains("application/pdf"));
    assert_eq!(
        delivery::history(&store, &a, &c, &d.id, 1)
            .await
            .unwrap()
            .items[0]
            .status,
        "sent"
    );
}

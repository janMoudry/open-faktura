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

use base64::Engine;
use faktura_application::attachment::{self, AttachmentStore, Upload};
use faktura_infrastructure::attachment::PgAttachment;
fn upload(name: &str, bytes: &[u8], key: &str) -> Upload {
    Upload {
        request_id: key.into(),
        filename: name.into(),
        base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn limits_scope_replay_delete_and_roles(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let d = issued(&pool, &a, &c).await;
    let s = PgAttachment(pool.clone());
    let (one, two) = tokio::join!(
        attachment::upload(
            &s,
            &a,
            &c,
            "invoice",
            &d.id,
            upload("note.txt", b"private text", KEY)
        ),
        attachment::upload(
            &s,
            &a,
            &c,
            "invoice",
            &d.id,
            upload("note.txt", b"private text", KEY)
        )
    );
    let file = one.unwrap();
    assert_eq!(file.id, two.unwrap().id);
    assert!(matches!(
        attachment::upload(
            &s,
            &a,
            &c,
            "invoice",
            &d.id,
            upload("changed.txt", b"private text", KEY)
        )
        .await,
        Err(Error::Conflict)
    ));
    assert!(s
        .download(&a, &other, "invoice", &d.id, &file.id, None)
        .await
        .is_err());
    assert!(s
        .download(&a, &c, "quote", &d.id, &file.id, None)
        .await
        .is_err());
    for (name, bytes) in [
        ("../bad.txt", b"hello".as_slice()),
        ("bad.exe", b"MZ"),
        ("fake.pdf", b"hello"),
        ("empty.txt", b""),
    ] {
        assert!(attachment::upload(
            &s,
            &a,
            &c,
            "invoice",
            &d.id,
            upload(name, bytes, "00000000-0000-4000-8000-000000000099")
        )
        .await
        .is_err());
    }
    assert!(attachment::upload(
        &s,
        &a,
        &c,
        "invoice",
        &d.id,
        upload(
            "big.txt",
            &vec![b'a'; 2097153],
            "00000000-0000-4000-8000-000000000099"
        )
    )
    .await
    .is_err());
    for i in 2..=5 {
        attachment::upload(
            &s,
            &a,
            &c,
            "invoice",
            &d.id,
            upload(
                "note.txt",
                b"second",
                &format!("00000000-0000-4000-8000-{i:012}"),
            ),
        )
        .await
        .unwrap();
    }
    assert!(attachment::upload(
        &s,
        &a,
        &c,
        "invoice",
        &d.id,
        upload(
            "sixth.txt",
            b"sixth",
            "00000000-0000-4000-8000-000000000006"
        )
    )
    .await
    .is_err());
    s.delete(&a, &c, "invoice", &d.id, &file.id).await.unwrap();
    s.delete(&a, &c, "invoice", &d.id, &file.id).await.unwrap();
    assert!(s
        .download(&a, &c, "invoice", &d.id, &file.id, None)
        .await
        .is_err());
    let removed: bool =
        sqlx::query_scalar("SELECT content IS NULL FROM document_attachment WHERE id=$1")
            .bind(file.id.parse::<i64>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(removed);
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let files = s.list(&a, &c, "invoice", &d.id).await.unwrap();
    assert_eq!(files.len(), 4);
    assert!(s
        .download(&a, &c, "invoice", &d.id, &files[0].id, None)
        .await
        .is_ok());
    assert!(s
        .delete(&a, &c, "invoice", &d.id, &files[0].id)
        .await
        .is_err());
    assert!(attachment::upload(
        &s,
        &a,
        &c,
        "invoice",
        &d.id,
        upload("new.txt", b"new", "00000000-0000-4000-8000-000000000007")
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn invoice_email_retains_selected_files_after_deletion(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let d = issued(&pool, &a, &c).await;
    let s = PgAttachment(pool.clone());
    let file = attachment::upload(
        &s,
        &a,
        &c,
        "invoice",
        &d.id,
        upload("specification.txt", b"immutable custom payload", KEY),
    )
    .await
    .unwrap();
    let excluded = attachment::upload(
        &s,
        &a,
        &c,
        "invoice",
        &d.id,
        upload(
            "private.txt",
            b"do not send",
            "00000000-0000-4000-8000-000000000002",
        ),
    )
    .await
    .unwrap();
    let (port, capture) = smtp(false).await;
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let mut cmd = command(d.revision, KEY);
    cmd.attachment_ids = vec![file.id.clone()];
    let sent = invoice_delivery::send(&mail, &a, &c, &d.id, cmd.clone())
        .await
        .unwrap();
    assert_eq!(sent.attachments.len(), 1);
    s.delete(&a, &c, "invoice", &d.id, &file.id).await.unwrap();
    assert_eq!(
        invoice_delivery::send(&mail, &a, &c, &d.id, cmd)
            .await
            .unwrap()
            .id,
        sent.id
    );
    assert_eq!(
        s.download(&a, &c, "invoice", &d.id, &file.id, Some(&sent.id))
            .await
            .unwrap()
            .content,
        b"immutable custom payload"
    );
    assert!(s
        .download(&a, &other, "invoice", &d.id, &file.id, Some(&sent.id))
        .await
        .is_err());
    assert!(s
        .download(&a, &c, "invoice", &d.id, &excluded.id, Some(&sent.id))
        .await
        .is_err());
    assert!(mail.process_one().await.unwrap());
    let message = capture.await.unwrap();
    assert!(message.contains("specification.txt"));
    assert!(!message.contains("private.txt"));
    assert!(message.contains("application/pdf"));
    let mut cmd = command(d.revision, "00000000-0000-4000-8000-000000000003");
    cmd.attachment_ids = vec![file.id];
    assert!(invoice_delivery::send(&mail, &a, &c, &d.id, cmd)
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn quote_attachments_and_rest_mcp_download(pool: PgPool) {
    use faktura_application::sales::{self, Data, Kind};
    use faktura_infrastructure::sales::PgSales;
    let (a, token, c, _) = setup(&pool).await;
    let quote = sales::create(
        &PgSales(pool.clone()),
        &a,
        &c,
        sales::Create {
            request_id: KEY.into(),
            kind: Kind::Quote,
            data: Data {
                name: "Synthetic quote".into(),
                maturity_days: 14,
                status: "created".into(),
                invoice: data(),
            },
        },
    )
    .await
    .unwrap();
    let s = PgAttachment(pool.clone());
    let file = attachment::upload(
        &s,
        &a,
        &c,
        "quote",
        &quote.id,
        upload("manual.txt", b"quote manual", KEY),
    )
    .await
    .unwrap();
    let (port, capture) = smtp(false).await;
    let mail = PgDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let sent = delivery::send(
        &mail,
        &a,
        &c,
        &quote.id,
        delivery::SendQuote {
            attachment_ids: vec![file.id.clone()],
            request_id: KEY.into(),
            revision: quote.revision,
            recipient: "customer@example.test".into(),
            subject: "Quote".into(),
            body: "Attached".into(),
            share_id: None,
        },
    )
    .await
    .unwrap();
    s.delete(&a, &c, "quote", &quote.id, &file.id)
        .await
        .unwrap();
    assert!(mail.process_one().await.unwrap());
    assert!(capture.await.unwrap().contains("manual.txt"));
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let app = faktura_server::attachment::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let path = format!(
        "/api/companies/{c}/attachments/quote/{}/deliveries/{}/{}",
        quote.id, sent.id, file.id
    );
    let r = app
        .clone()
        .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&path)
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["x-content-type-options"], "nosniff");
    assert!(r.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .starts_with("attachment;"));
    assert_eq!(
        to_bytes(r.into_body(), 1000).await.unwrap(),
        b"quote manual".as_slice()
    );
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    let server = faktura_server::mcp::CompanyMcp::new(pool, token);
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
    let args = serde_json::json!({"company_id":c,"kind":"quote","document_id":quote.id,"id":file.id,"delivery":sent.id});
    let r = client
        .call_tool(
            CallToolRequestParams::new("attachment_historydownload")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(r.is_error, Some(false));
    let v: serde_json::Value = serde_json::from_str(&r.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(
        v["base64"],
        base64::engine::general_purpose::STANDARD.encode(b"quote manual")
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

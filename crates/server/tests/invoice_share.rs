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
use faktura_infrastructure::{delivery::MailConfig, invoice_delivery::PgInvoiceDelivery};
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

use faktura_application::{
    draft::DraftStore,
    payment::{self, PaymentStore},
    share::{self},
};
use faktura_infrastructure::{invoice_share::PgInvoiceShare, payment::PgPayment};
fn shares(pool: &PgPool) -> PgInvoiceShare {
    PgInvoiceShare {
        pool: pool.clone(),
        base_url: Some("https://invoice.example.test".into()),
    }
}
fn token(link: &share::Link) -> &str {
    link.url.rsplit('/').next().unwrap()
}
fn create_link(revision: i64) -> share::Create {
    share::Create {
        request_id: KEY.into(),
        revision,
        expires_days: 30,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_links_pdf_views_settlement_and_revocation(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c).await;
    let s = shares(&pool);
    let (x, y) = tokio::join!(
        share::create(&s, &a, &c, &invoice.id, create_link(invoice.revision)),
        share::create(&s, &a, &c, &invoice.id, create_link(invoice.revision))
    );
    let link = x.unwrap();
    assert_eq!(link.url, y.unwrap().url);
    assert!(link.url.contains("/#/shared/invoice/"));
    assert!(matches!(
        share::create(
            &s,
            &a,
            &c,
            &invoice.id,
            share::Create {
                expires_days: 31,
                ..create_link(invoice.revision)
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    let original = PgDraft(pool.clone())
        .pdf(&a, &c, &invoice.id)
        .await
        .unwrap();
    assert_eq!(share::public_pdf(&s, token(&link)).await.unwrap(), original);
    let public = share::public_quote(&s, token(&link)).await.unwrap();
    assert_eq!(public.status, "unpaid");
    assert!(public.invoice.contact_id.is_none());
    let payment_store = PgPayment(pool.clone());
    let p = payment_store.get(&a, &c, &invoice.id).await.unwrap();
    payment::register(
        &payment_store,
        &a,
        &c,
        &invoice.id,
        payment::Register {
            request_id: KEY.into(),
            revision: p.summary.revision,
            payment: payment::PaymentInput {
                amount: "59.99".into(),
                paid_on: p.summary.today.parse().unwrap(),
                method: "cash".into(),
                note: "PRIVATE PAYMENT NOTE".into(),
            },
        },
    )
    .await
    .unwrap();
    let v = share::public_quote(&s, token(&link)).await.unwrap();
    assert_eq!(v.status, "paid");
    let json = serde_json::to_value(&v).unwrap();
    assert!(!json.to_string().contains("PRIVATE PAYMENT NOTE"));
    assert!(json.get("company_id").is_none());
    assert!(json.get("settlement").is_none());
    assert_eq!(share::public_pdf(&s, token(&link)).await.unwrap(), original);
    share::view(&s, token(&link), KEY, invoice.revision)
        .await
        .unwrap();
    share::view(&s, token(&link), KEY, invoice.revision)
        .await
        .unwrap();
    assert_eq!(
        share::views(&s, &a, &c, &invoice.id, &link.id, 1)
            .await
            .unwrap()
            .total,
        1
    );
    for _ in 0..65 {
        share::view(
            &s,
            token(&link),
            &uuid::Uuid::new_v4().to_string(),
            invoice.revision,
        )
        .await
        .unwrap();
    }
    assert_eq!(
        share::views(&s, &a, &c, &invoice.id, &link.id, 1)
            .await
            .unwrap()
            .total,
        60
    );
    assert!(share::views(&s, &a, &other, &invoice.id, &link.id, 1)
        .await
        .is_err());
    share::revoke(&s, &a, &c, &invoice.id, &link.id)
        .await
        .unwrap();
    share::revoke(&s, &a, &c, &invoice.id, &link.id)
        .await
        .unwrap();
    assert!(matches!(
        share::public_pdf(&s, token(&link)).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        share::public_quote(&s, token(&link)).await,
        Err(Error::NotFound)
    ));
    assert!(share::view(&s, token(&link), KEY, invoice.revision)
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn link_email_and_revoked_queue_preserve_attachment_history(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c).await;
    let s = shares(&pool);
    let link = share::create(&s, &a, &c, &invoice.id, create_link(invoice.revision))
        .await
        .unwrap();
    let (port, capture) = smtp(false).await;
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let mut cmd = command(invoice.revision, KEY);
    cmd.share_id = Some(link.id.clone());
    let d = invoice_delivery::send(&mail, &a, &c, &invoice.id, cmd.clone())
        .await
        .unwrap();
    assert_eq!(d.share_id.as_deref(), Some(link.id.as_str()));
    assert!(d.body.contains(&link.url));
    assert_eq!(
        invoice_delivery::send(&mail, &a, &c, &invoice.id, cmd)
            .await
            .unwrap()
            .id,
        d.id
    );
    assert!(mail.process_one().await.unwrap());
    let mime = capture.await.unwrap();
    assert!(!mime.contains("application/pdf"));
    assert_eq!(
        delivery::pdf(&mail, &a, &c, &invoice.id, Some(&d.id))
            .await
            .unwrap(),
        share::public_pdf(&s, token(&link)).await.unwrap()
    );
    let mut cmd = command(invoice.revision, &uuid::Uuid::new_v4().to_string());
    cmd.share_id = Some(link.id.clone());
    invoice_delivery::send(&mail, &a, &c, &invoice.id, cmd)
        .await
        .unwrap();
    share::revoke(&s, &a, &c, &invoice.id, &link.id)
        .await
        .unwrap();
    mail.process_one().await.unwrap();
    assert_eq!(
        delivery::history(&mail, &a, &c, &invoice.id, 1)
            .await
            .unwrap()
            .items[0]
            .error_code
            .as_deref(),
        Some("link_unavailable")
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn scope_roles_expiry_draft_and_deleted_company(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c).await;
    let s = shares(&pool);
    let draft = draft::create(&PgDraft(pool.clone()), &a, &other, create(KEY))
        .await
        .unwrap();
    assert!(
        share::create(&s, &a, &other, &draft.id, create_link(draft.revision))
            .await
            .is_err()
    );
    assert!(
        share::create(&s, &a, &other, &invoice.id, create_link(invoice.revision))
            .await
            .is_err()
    );
    assert!(share::create(
        &s,
        &a,
        &c,
        &invoice.id,
        share::Create {
            expires_days: 0,
            ..create_link(invoice.revision)
        }
    )
    .await
    .is_err());
    let link = share::create(&s, &a, &c, &invoice.id, create_link(invoice.revision))
        .await
        .unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(share::list(&s, &a, &c, &invoice.id, 1).await.is_ok());
    assert!(matches!(
        share::revoke(&s, &a, &c, &invoice.id, &link.id).await,
        Err(Error::Forbidden)
    ));
    sqlx::query("UPDATE invoice_share SET expires_at=now()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        share::public_quote(&s, token(&link)).await,
        Err(Error::NotFound)
    ));
    sqlx::query("UPDATE invoice_share SET expires_at=now()+interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE company SET deleted_at=now() WHERE id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        share::public_pdf(&s, token(&link)).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        share::public_quote(&s, "bad-token").await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL REST MCP"]
async fn rest_mcp_public_headers_and_same_origin(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt as _;
    let (a, session, c, _) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c).await;
    let s = shares(&pool);
    let link = share::create(&s, &a, &c, &invoice.id, create_link(invoice.revision))
        .await
        .unwrap();
    let app = faktura_server::invoice_share::router(
        s.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/public/invoices/resolve")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"token":token(&link)}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["cache-control"], "no-store");
    assert_eq!(r.headers()["referrer-policy"], "no-referrer");
    assert!(r.headers()["x-robots-tag"]
        .to_str()
        .unwrap()
        .contains("noindex"));
    let v: serde_json::Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
    assert_eq!(v["number"], invoice.number.unwrap());
    assert!(v.get("id").is_none());
    let r = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/companies/{c}/invoices/{}/shares/{}/revoke",
                    invoice.id, link.id
                ))
                .header("cookie", format!("faktura_session={session}"))
                .header("origin", "https://other.example.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), session).with_share(
        faktura_infrastructure::share::PgShare {
            pool: pool.clone(),
            base_url: s.base_url,
        },
    );
    let task = tokio::spawn(async move {
        server
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ().serve(client_io).await.unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("invoice_sharelist").with_arguments(
                serde_json::json!({"company_id":c,"id":invoice.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_023_preserves_issued_pdf_and_mail_replay(pool: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610130024)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c).await;
    let bytes = PgDraft(pool.clone())
        .pdf(&a, &c, &invoice.id)
        .await
        .unwrap();
    let cmd = command(invoice.revision, KEY);
    sqlx::query("INSERT INTO invoice_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id,status) VALUES($1,$2,$3,$4::text::uuid,$5,$6,'{}',$7,'sender@example.test','','customer@example.test','Old mail','Old body','old-share@example.test','sent')").bind(c.parse::<i64>().unwrap()).bind(invoice.id.parse::<i64>().unwrap()).bind(a.parse::<i64>().unwrap()).bind(KEY).bind(serde_json::to_value(&cmd).unwrap()).bind(invoice.revision).bind(&bytes).execute(&pool).await.unwrap();
    faktura_infrastructure::migrate(&pool).await.unwrap();
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: None,
    };
    assert_eq!(
        invoice_delivery::send(&mail, &a, &c, &invoice.id, cmd)
            .await
            .unwrap()
            .status,
        "sent"
    );
    let s = shares(&pool);
    let link = share::create(&s, &a, &c, &invoice.id, create_link(invoice.revision))
        .await
        .unwrap();
    assert_eq!(share::public_pdf(&s, token(&link)).await.unwrap(), bytes);
}

use faktura_application::{
    auth, company,
    draft::{self, Create, DraftStore, Error, Issue},
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
    d.due_on = Some("2026-10-11".parse().unwrap());
    d.customer.as_mut().unwrap().email = "customer@example.test".into();
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
    communication::{CommunicationStore, Pause, Settings},
    payment::{self, PaymentStore},
};
use faktura_infrastructure::{communication::PgCommunication, payment::PgPayment};
async fn pay(pool: &PgPool, a: &str, c: &str, i: &str, amount: &str) -> payment::Payments {
    let store = PgPayment(pool.clone());
    let p = store.get(a, c, i).await.unwrap();
    payment::register(
        &store,
        a,
        c,
        i,
        payment::Register {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: p.summary.revision,
            payment: payment::PaymentInput {
                amount: amount.into(),
                paid_on: p.summary.today.parse().unwrap(),
                method: "cash".into(),
                note: String::new(),
            },
        },
    )
    .await
    .unwrap()
}
async fn mail_command(
    store: &PgCommunication,
    a: &str,
    c: &str,
    i: &str,
    purpose: &str,
) -> SendInvoice {
    let p = store.preview(a, c, i, purpose).await.unwrap();
    let mut v = command(p.revision, &uuid::Uuid::new_v4().to_string());
    v.purpose = Some(purpose.into());
    v.settlement_revision = Some(p.settlement_revision);
    v.recipient = p.recipient;
    v.subject = p.subject;
    v.body = p.body;
    v
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn manual_reminder_partial_payment_replay_and_dispatch_guard(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let store = PgCommunication(pool.clone());
    let p = store.preview(&a, &c, &i.id, "reminder").await.unwrap();
    assert!(p.eligible);
    assert!(p.subject.contains("Upomínka"));
    assert!(p.body.contains("59,99"));
    assert!(matches!(
        store.preview(&a, &other, &i.id, "reminder").await,
        Err(Error::NotFound)
    ));
    let (port, capture) = smtp(false).await;
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let cmd = mail_command(&store, &a, &c, &i.id, "reminder").await;
    let (x, y) = tokio::join!(
        invoice_delivery::send(&mail, &a, &c, &i.id, cmd.clone()),
        invoice_delivery::send(&mail, &a, &c, &i.id, cmd.clone())
    );
    assert_eq!(x.unwrap().id, y.unwrap().id);
    pay(&pool, &a, &c, &i.id, "10.00").await;
    assert!(mail.process_one().await.unwrap());
    let h = delivery::history(&mail, &a, &c, &i.id, 1).await.unwrap();
    assert_eq!(h.items[0].status, "failed");
    assert_eq!(
        h.items[0].error_code.as_deref(),
        Some("communication_changed")
    );
    let cmd = mail_command(&store, &a, &c, &i.id, "reminder").await;
    assert!(cmd.body.contains("49,99"));
    invoice_delivery::send(&mail, &a, &c, &i.id, cmd)
        .await
        .unwrap();
    mail.process_one().await.unwrap();
    let mime = capture.await.unwrap();
    assert!(mime.contains("application/pdf"));
    assert_eq!(
        delivery::history(&mail, &a, &c, &i.id, 1)
            .await
            .unwrap()
            .items[0]
            .purpose,
        "reminder"
    );
    assert_eq!(
        PgDraft(pool.clone())
            .get(&a, &c, &i.id)
            .await
            .unwrap()
            .revision,
        i.revision
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn thanks_requires_cash_and_full_payment_and_cancels_after_reversal(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let store = PgCommunication(pool.clone());
    assert!(
        !store
            .preview(&a, &c, &i.id, "thanks")
            .await
            .unwrap()
            .eligible
    );
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(9))),
    };
    assert!(matches!(
        invoice_delivery::send(
            &mail,
            &a,
            &c,
            &i.id,
            mail_command(&store, &a, &c, &i.id, "thanks").await
        )
        .await,
        Err(Error::Conflict)
    ));
    let p = pay(&pool, &a, &c, &i.id, "59.99").await;
    let cmd = mail_command(&store, &a, &c, &i.id, "thanks").await;
    invoice_delivery::send(&mail, &a, &c, &i.id, cmd.clone())
        .await
        .unwrap();
    payment::reverse(
        &PgPayment(pool.clone()),
        &a,
        &c,
        &i.id,
        &p.items[0].id,
        payment::Reverse {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: p.summary.revision,
            reason: "Synthetic reversal".into(),
            replacement: None,
        },
    )
    .await
    .unwrap();
    mail.process_one().await.unwrap();
    assert_eq!(
        delivery::history(&mail, &a, &c, &i.id, 1)
            .await
            .unwrap()
            .items[0]
            .error_code
            .as_deref(),
        Some("communication_changed")
    );
    // Identical request returns historical result, changed key rejects stale settlement.
    invoice_delivery::send(&mail, &a, &c, &i.id, cmd.clone())
        .await
        .unwrap();
    let mut stale = cmd;
    stale.request_id = uuid::Uuid::new_v4().to_string();
    assert!(matches!(
        invoice_delivery::send(&mail, &a, &c, &i.id, stale).await,
        Err(Error::Conflict)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn automatic_intervals_threshold_pause_limit_and_settings_revision(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let store = PgCommunication(pool.clone());
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(9))),
    };
    assert!(!store.process_one(&mail).await.unwrap());
    let mut s = store
        .save(
            &a,
            &c,
            Settings {
                reminders: true,
                first_days: 1,
                repeat_days: 1,
                minimum_total: "60.00".into(),
                ..Settings::default()
            },
        )
        .await
        .unwrap();
    assert!(!store.process_one(&mail).await.unwrap());
    s.minimum_total = "59.99".into();
    s = store.save(&a, &c, s).await.unwrap();
    assert!(!store.process_one(&mail).await.unwrap());
    s.minimum_total = "0.00".into();
    s = store.save(&a, &c, s).await.unwrap();
    store
        .pause(&a, &c, &i.id, Pause { paused: true })
        .await
        .unwrap();
    assert!(!store.process_one(&mail).await.unwrap());
    store
        .pause(&a, &c, &i.id, Pause { paused: false })
        .await
        .unwrap();
    let (x, y) = tokio::join!(store.process_one(&mail), store.process_one(&mail));
    x.unwrap();
    y.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invoice_delivery")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(!store.process_one(&mail).await.unwrap());
    // Disabling cancels existing queued communication; no SMTP connection is attempted.
    s.reminders = false;
    s = store.save(&a, &c, s).await.unwrap();
    mail.process_one().await.unwrap();
    assert_eq!(
        delivery::history(&mail, &a, &c, &i.id, 1)
            .await
            .unwrap()
            .items[0]
            .error_code
            .as_deref(),
        Some("communication_changed")
    );
    s.reminders = true;
    store.save(&a, &c, s).await.unwrap();
    // Synthetic clock progression by aging intents; invoice remains overdue.
    for _ in 1..10 {
        sqlx::query("UPDATE communication_job SET created_at=now()-interval '2 days'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(store.process_one(&mail).await.unwrap());
    }
    sqlx::query("UPDATE communication_job SET created_at=now()-interval '2 days'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!store.process_one(&mail).await.unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM communication_job")
            .fetch_one(&pool)
            .await
            .unwrap(),
        10
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn automatic_thanks_new_payments_once_and_english_preview(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let old = issued(&pool, &a, &other).await;
    pay(&pool, &a, &other, &old.id, "59.99").await;
    let store = PgCommunication(pool.clone());
    store
        .save(
            &a,
            &other,
            Settings {
                thanks: true,
                ..Settings::default()
            },
        )
        .await
        .unwrap();
    store
        .save(
            &a,
            &c,
            Settings {
                thanks: true,
                ..Settings::default()
            },
        )
        .await
        .unwrap();
    let (port, capture) = smtp(false).await;
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    assert!(!store.process_one(&mail).await.unwrap());
    pay(&pool, &a, &c, &i.id, "10.00").await;
    assert!(!store.process_one(&mail).await.unwrap());
    pay(&pool, &a, &c, &i.id, "49.99").await;
    store.process_one(&mail).await.unwrap();
    assert!(!store.process_one(&mail).await.unwrap());
    mail.process_one().await.unwrap();
    assert!(capture.await.unwrap().contains("application/pdf"));
    assert_eq!(
        delivery::history(&mail, &a, &c, &i.id, 1)
            .await
            .unwrap()
            .items[0]
            .purpose,
        "thanks"
    );
    // English is a document choice, independent from the UI language.
    let mut d = create(&uuid::Uuid::new_v4().to_string());
    d.data.language = "en".into();
    let d = draft::create(&PgDraft(pool.clone()), &a, &c, d)
        .await
        .unwrap();
    let en = draft::issue(
        &PgDraft(pool.clone()),
        &a,
        &c,
        &d.id,
        Issue {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    assert!(store
        .preview(&a, &c, &en.id, "reminder")
        .await
        .unwrap()
        .subject
        .starts_with("Payment reminder"));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn settings_roles_stale_validation_rest_and_mcp(pool: PgPool) {
    let (a, token, c, other) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let store = PgCommunication(pool.clone());
    assert!(store
        .save(
            &a,
            &c,
            Settings {
                first_days: 0,
                ..Settings::default()
            }
        )
        .await
        .is_err());
    store.save(&a, &c, Settings::default()).await.unwrap();
    assert!(matches!(
        store.save(&a, &c, Settings::default()).await,
        Err(Error::Conflict)
    ));
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let app = faktura_server::communication::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let req = |uri: String| {
        Request::builder()
            .uri(uri)
            .header("cookie", format!("faktura_session={token}"))
            .body(Body::empty())
            .unwrap()
    };
    let response = app
        .clone()
        .oneshot(req(format!(
            "/api/companies/{c}/invoices/{}/communication/reminder",
            i.id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let response = app
        .oneshot(req(format!(
            "/api/companies/{other}/invoices/{}/communication/reminder",
            i.id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token);
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
    let preview = client
        .call_tool(
            CallToolRequestParams::new("communication_preview").with_arguments(
                serde_json::json!({"company_id":c,"id":i.id,"purpose":"reminder"})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(preview.is_error, Some(true));
    client.cancel().await.unwrap();
    task.await.unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.settings(&a, &c).await.is_ok());
    assert!(matches!(
        store.pause(&a, &c, &i.id, Pause { paused: true }).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store
            .save(
                &a,
                &c,
                Settings {
                    revision: 1,
                    ..Settings::default()
                }
            )
            .await,
        Err(Error::Forbidden)
    ));
    sqlx::query("UPDATE membership SET role='editor' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store
            .save(
                &a,
                &c,
                Settings {
                    revision: 1,
                    ..Settings::default()
                }
            )
            .await,
        Err(Error::Forbidden)
    ));
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_022_preserves_pdf_payments_and_existing_delivery(pool: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610130023)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let original = PgDraft(pool.clone()).pdf(&a, &c, &i.id).await.unwrap();
    // Insert the exact old row shape before the additive schema is available.
    sqlx::query("INSERT INTO invoice_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id,status) VALUES($1,$2,$3,$4::text::uuid,'{}',$5,'{}',$6,'sender@example.test','','customer@example.test','Old mail','Old body','old@example.test','sent')")
 .bind(c.parse::<i64>().unwrap()).bind(i.id.parse::<i64>().unwrap()).bind(a.parse::<i64>().unwrap()).bind(KEY).bind(i.revision).bind(&original).execute(&pool).await.unwrap();
    pay(&pool, &a, &c, &i.id, "10.00").await;
    faktura_infrastructure::migrate(&pool).await.unwrap();
    assert_eq!(
        PgDraft(pool.clone()).pdf(&a, &c, &i.id).await.unwrap(),
        original
    );
    assert_eq!(
        PgPayment(pool.clone())
            .get(&a, &c, &i.id)
            .await
            .unwrap()
            .summary
            .outstanding,
        "49.99"
    );
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: None,
    };
    let h = delivery::history(&mail, &a, &c, &i.id, 1).await.unwrap();
    assert_eq!(h.items[0].purpose, "document");
    assert_eq!(h.items[0].status, "sent");
    assert!(
        !PgCommunication(pool)
            .settings(&a, &c)
            .await
            .unwrap()
            .reminders
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL dispatch authorization"]
async fn queued_automation_pause_and_revoked_actor_cancel_before_smtp(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let i = issued(&pool, &a, &c).await;
    let store = PgCommunication(pool.clone());
    store
        .save(
            &a,
            &c,
            Settings {
                reminders: true,
                first_days: 1,
                ..Settings::default()
            },
        )
        .await
        .unwrap();
    let mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(9))),
    };
    store.process_one(&mail).await.unwrap();
    store
        .pause(&a, &c, &i.id, Pause { paused: true })
        .await
        .unwrap();
    mail.process_one().await.unwrap();
    assert_eq!(
        delivery::history(&mail, &a, &c, &i.id, 1)
            .await
            .unwrap()
            .items[0]
            .error_code
            .as_deref(),
        Some("communication_changed")
    );
    let cmd = mail_command(&store, &a, &c, &i.id, "reminder").await;
    invoice_delivery::send(&mail, &a, &c, &i.id, cmd)
        .await
        .unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    mail.process_one().await.unwrap();
    assert_eq!(
        delivery::history(&mail, &a, &c, &i.id, 1)
            .await
            .unwrap()
            .items[0]
            .error_code
            .as_deref(),
        Some("communication_changed")
    );
}

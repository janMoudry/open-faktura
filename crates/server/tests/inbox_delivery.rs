use base64::Engine;
use faktura_application::{
    auth, company,
    inbox::{InboxStore, Upload},
    inbox_delivery::{InboxDeliveryStore, Send},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{
    auth::PgAuth, company::PgCompany, delivery::MailConfig, inbox::PgInbox,
    inbox_delivery::PgInboxDelivery,
};
use sqlx::PgPool;
use std::sync::Arc;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
async fn setup(p: &PgPool) -> (String, String, String, String) {
    let s = PgAuth::new(p.clone(), Some("synthetic-original-mail".into()));
    auth::bootstrap(
        &s,
        "synthetic-original-mail",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, t) = auth::login(&s, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let c = company::create(
        &PgCompany(p.clone()),
        &a.id,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let d = PgInbox(p.clone())
        .upload(
            &a.id,
            &c.id,
            Upload {
                request_id: key(),
                filename: "original.pdf".into(),
                base64: base64::engine::general_purpose::STANDARD
                    .encode(b"%PDF-1.4\nsynthetic original bytes"),
            },
        )
        .await
        .unwrap();
    (a.id, t, c.id, d.id)
}
fn cmd() -> Send {
    Send {
        request_id: key(),
        revision: 1,
        recipient: "accountant@example.test".into(),
        subject: "Synthetic original".into(),
        body: "Synthetic accounting test only".into(),
        confirm_original: true,
    }
}
async fn capture(reject: bool) -> (u16, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
        let (s, _) = l.accept().await.unwrap();
        let (r, mut w) = s.into_split();
        let mut r = BufReader::new(r);
        w.write_all(b"220 synthetic ESMTP\r\n").await.unwrap();
        let (mut data, mut out) = (false, String::new());
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            if data {
                if line == ".\r\n" {
                    w.write_all(b"250 accepted\r\n").await.unwrap();
                    break;
                }
                out.push_str(&line);
                continue;
            }
            let reply: &[u8] = if line.starts_with("EHLO") {
                b"250 synthetic\r\n"
            } else if line.starts_with("DATA") {
                data = true;
                b"354 proceed\r\n"
            } else if reject && line.starts_with("RCPT") {
                b"550 rejected\r\n"
            } else {
                b"250 ok\r\n"
            };
            w.write_all(reply).await.unwrap();
            if reject && line.starts_with("RCPT") {
                break;
            }
        }
        out
    });
    (port, task)
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated DB and synthetic loopback SMTP"]
async fn original_snapshot_explicit_ack_replay_headers_scope_rest_mcp_and_single_dispatch(
    p: PgPool,
) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let (a, token, c, d) = setup(&p).await;
    let (port, capture) = capture(false).await;
    let s = PgInboxDelivery {
        pool: p.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let mut bad = cmd();
    bad.confirm_original = false;
    assert!(s.send(&a, &c, &d, bad).await.is_err());
    let mut bad = cmd();
    bad.recipient = "accountant@example.test\r\nBcc: other@example.test".into();
    assert!(s.send(&a, &c, &d, bad).await.is_err());
    let mut bad = cmd();
    bad.subject = "bad\nheader".into();
    assert!(s.send(&a, &c, &d, bad).await.is_err());
    assert!(s.send(&a, &c, "999999", cmd()).await.is_err());
    let other = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    assert!(s.send(&a, &other.id, &d, cmd()).await.is_err());
    let command = cmd();
    let queued = s.send(&a, &c, &d, command.clone()).await.unwrap();
    assert_eq!(queued.status, "queued");
    assert_eq!(
        s.send(&a, &c, &d, command.clone()).await.unwrap().id,
        queued.id
    );
    let mut changed = command.clone();
    changed.recipient = "different@example.test".into();
    assert!(s.send(&a, &c, &d, changed).await.is_err());
    let app = faktura_server::inbox_delivery::router(
        p.clone(),
        vec!["http://localhost:5173".into()],
        false,
        s.clone(),
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{c}/inbox/{d}/deliveries"))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let rest: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    let args = serde_json::json!({"company_id":c,"id":d})
        .as_object()
        .unwrap()
        .clone();
    let tool = faktura_server::inbox_delivery::tool(
        p.clone(),
        s.mail.clone(),
        &a,
        "inbox_delivery_history",
        args,
    )
    .await
    .unwrap();
    assert_eq!(rest, tool);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{c}/inbox/{d}/deliveries"))
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("X-Faktura-Request", "1")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&command).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let restsend: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    let mut args = serde_json::to_value(&command)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    args.insert("company_id".into(), c.clone().into());
    args.insert("id".into(), d.clone().into());
    let toolsend = faktura_server::inbox_delivery::tool(
        p.clone(),
        s.mail.clone(),
        &a,
        "inbox_delivery_send",
        args,
    )
    .await
    .unwrap();
    assert_eq!(restsend, toolsend);
    assert_eq!(toolsend["id"], queued.id);

    // Archive/revision changes do not rewrite the already confirmed immutable file snapshot.
    PgInbox(p.clone())
        .execute(
            &a,
            &c,
            faktura_application::inbox::Command {
                request_id: key(),
                id: d.clone(),
                revision: 1,
                action: faktura_application::inbox::Action::Archive { archived: true },
            },
        )
        .await
        .unwrap();
    let (one, two) = tokio::join!(s.process_one(), s.process_one());
    assert_eq!(usize::from(one.unwrap()) + usize::from(two.unwrap()), 1);
    let message = tokio::time::timeout(std::time::Duration::from_secs(5), capture)
        .await
        .unwrap()
        .unwrap();
    assert!(message.contains("accountant@example.test"));
    assert!(
        message.contains("filename=original.pdf") || message.contains("filename=\"original.pdf\"")
    );
    assert!(message.contains(
        &base64::engine::general_purpose::STANDARD.encode(b"%PDF-1.4\nsynthetic original bytes")
    ));
    assert_eq!(
        message.matches("Content-Disposition: attachment").count(),
        1
    );
    let h = s.history(&a, &c, &d).await.unwrap();
    assert_eq!(h.items.len(), 1);
    assert_eq!(h.items[0].status, "sent");
    assert_eq!(s.send(&a, &c, &d, command).await.unwrap().id, queued.id);
    assert!(!s.process_one().await.unwrap());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated DB and synthetic loopback SMTP"]
async fn revoked_permission_failed_rejected_and_uncertain_never_auto_retry(p: PgPool) {
    let (a, _, c, d) = setup(&p).await;
    let (port, capture) = capture(true).await;
    let s = PgInboxDelivery {
        pool: p.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let command = cmd();
    s.send(&a, &c, &d, command).await.unwrap();
    assert!(s.process_one().await.unwrap());
    capture.await.unwrap();
    assert_eq!(
        s.history(&a, &c, &d).await.unwrap().items[0].status,
        "failed"
    );
    assert!(!s.process_one().await.unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let s = PgInboxDelivery {
        pool: p.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(
            listener.local_addr().unwrap().port(),
        ))),
    };
    let queued = s.send(&a, &c, &d, cmd()).await.unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&a).execute(&p).await.unwrap();
    assert!(s.process_one().await.unwrap());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
    let h = s.history(&a, &c, &d).await.unwrap();
    assert_eq!(h.items[0].error_code.as_deref(), Some("permission_changed"));
    assert!(s.send(&a, &c, &d, cmd()).await.is_err());
    // Recovery marks a lost process as uncertain; it cannot dispatch again.
    sqlx::query("UPDATE inbox_delivery SET status='sending',started_at=now()-interval '3 minutes' WHERE id=$1::text::bigint").bind(&queued.id).execute(&p).await.unwrap();
    assert!(!s.process_one().await.unwrap());
    assert_eq!(
        s.history(&a, &c, &d).await.unwrap().items[0].status,
        "uncertain"
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn queue_replay_and_history_with_one_pool_connection(p: PgPool) {
    let (actor, _, company, document) = setup(&p).await;
    let small = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(1))
        .connect_with((*p.connect_options()).clone())
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let store = PgInboxDelivery {
        pool: small.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(
            listener.local_addr().unwrap().port(),
        ))),
    };
    let command = cmd();
    let queued = store
        .send(&actor, &company, &document, command.clone())
        .await
        .unwrap();
    assert_eq!(
        store
            .send(&actor, &company, &document, command)
            .await
            .unwrap()
            .id,
        queued.id
    );
    assert_eq!(
        store
            .history(&actor, &company, &document)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    // Queuing must not contact SMTP; only the separately authorized worker sends.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
    small.close().await;
}

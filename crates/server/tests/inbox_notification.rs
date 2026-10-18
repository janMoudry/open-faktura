use base64::Engine;
use faktura_application::{
    auth, company,
    inbox::{InboxStore, Upload},
    inbox_notification::{Configure, NotificationsStore},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{
    auth::PgAuth, company::PgCompany, inbox::PgInbox, inbox_notification::PgNotifications,
};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn upload(content: &str) -> Upload {
    Upload {
        request_id: key(),
        filename: "safe.pdf".into(),
        base64: base64::engine::general_purpose::STANDARD.encode(format!("%PDF-1.4\n{content}")),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn private_opt_in_replay_safe_events_cursors_permissions_and_mcp(p: PgPool) {
    let auth = PgAuth::new(p.clone(), Some("synthetic-notifications".into()));
    auth::bootstrap(
        &auth,
        "synthetic-notifications",
        "notify@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, token) = auth::login(&auth, "notify@example.test", "Synthetic password 123!")
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
    let other = company::create(
        &PgCompany(p.clone()),
        &a.id,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let store = PgNotifications(p.clone());
    let inbox = PgInbox(p.clone());
    let old = inbox.upload(&a.id, &c.id, upload("old")).await.unwrap();
    let f = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM inbox_notification_settings")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
    assert!(f.events.is_empty());
    assert_eq!(f.unread, 0);
    assert!(!f.settings.new_documents);
    let f = store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 1,
                new_documents: true,
                errors: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(f.unread, 0);
    assert_eq!(f.events.len(), 1);
    assert!(store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 1,
                new_documents: false,
                errors: false
            }
        )
        .await
        .is_err());
    let input = upload("new");
    let d = inbox.upload(&a.id, &c.id, upload("new")).await.unwrap();
    assert_eq!(inbox.upload(&a.id, &c.id, input).await.unwrap().id, d.id);
    // Persisted MIME rejection generates only a stable code, never raw error details.
    sqlx::query("INSERT INTO inbox_mailbox_message(company_id,provider,uid_validity,uid,status,error) VALUES($1::text::bigint,'synthetic',1,1,'rejected','secret raw MIME or credentials')").bind(&c.id).execute(&p).await.unwrap();
    let f = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(f.unread, 2);
    assert_eq!(f.events.len(), 3);
    assert_eq!(f.events[0].code.as_deref(), Some("message_rejected"));
    assert!(!serde_json::to_string(&f).unwrap().contains("secret"));
    assert_eq!(store.feed(&a.id, &other.id).await.unwrap().unread, 0);
    assert!(store.read(&a.id, &other.id, &f.through).await.is_err());
    assert!(store.read(&a.id, &c.id, "999999999").await.is_err());
    let before = f.through.clone();
    inbox.upload(&a.id, &c.id, upload("later")).await.unwrap();
    assert_eq!(store.read(&a.id, &c.id, &before).await.unwrap().unread, 1);
    assert_eq!(store.read(&a.id, &c.id, &before).await.unwrap().unread, 1);
    // A read-only company member can configure only their own preferences/cursor.
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c.id).bind(&a.id).execute(&p).await.unwrap();
    let f = store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 2,
                new_documents: false,
                errors: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(f.unread, 0);
    assert!(f.events.iter().all(|e| e.category == "error"));
    let args = serde_json::json!({"company_id":c.id})
        .as_object()
        .unwrap()
        .clone();
    let tool =
        faktura_server::inbox_notification::tool(p.clone(), &a.id, "inbox_notification_feed", args)
            .await
            .unwrap();
    assert_eq!(tool, serde_json::to_value(&f).unwrap());
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let app = faktura_server::inbox_notification::router(
        p.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{}/inbox/notifications", c.id))
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(
            &to_bytes(response.into_body(), 100000).await.unwrap()
        )
        .unwrap(),
        tool
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/companies/{}/inbox/notifications", c.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/companies/{}/inbox/notifications/read", c.id))
                .header("cookie", format!("faktura_session={token}"))
                .header("content-type", "application/json")
                .body(Body::from("{\"through\":\"0\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let b:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('viewer@example.test','synthetic-not-a-password') RETURNING id").fetch_one(&p).await.unwrap();
    sqlx::query(
        "INSERT INTO membership(company_id,user_id,role) VALUES($1::text::bigint,$2,'viewer')",
    )
    .bind(&c.id)
    .bind(b)
    .execute(&p)
    .await
    .unwrap();
    let b = b.to_string();
    let bf = store.feed(&b, &c.id).await.unwrap();
    assert_eq!(bf.settings.revision, 1);
    assert_eq!(bf.unread, 0);
    assert!(bf.events.is_empty());
    let bf = store
        .configure(
            &b,
            &c.id,
            Configure {
                revision: 1,
                new_documents: false,
                errors: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(bf.unread, 0);
    sqlx::query("INSERT INTO inbox_mailbox_run(company_id,request_id,actor_id,settings_revision,status,error) VALUES($1::text::bigint,$2::text::uuid,$3::text::bigint,1,'failed','private raw error')").bind(&c.id).bind(key()).bind(&a.id).execute(&p).await.unwrap();
    assert_eq!(store.feed(&b, &c.id).await.unwrap().unread, 1);
    let af = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(af.unread, 1);
    assert_eq!(af.events[0].code.as_deref(), Some("mailbox_failed"));
    store.read(&b, &c.id, &af.through).await.unwrap();
    assert_eq!(store.feed(&a.id, &c.id).await.unwrap().unread, 1);

    let gateway_key = key();
    sqlx::query("INSERT INTO inbox_mail(company_id,message_id,request_id,actor_id,input_hash,sender,subject,status,attachment_count) VALUES($1::text::bigint,'synthetic-gateway',$2::text::uuid,$3::text::bigint,'synthetic','sender@example.test','synthetic','processing',0)").bind(&c.id).bind(&gateway_key).bind(&a.id).execute(&p).await.unwrap();
    sqlx::query("UPDATE inbox_mail SET status='failed' WHERE company_id=$1::text::bigint AND message_id='synthetic-gateway'").bind(&c.id).execute(&p).await.unwrap();
    sqlx::query("UPDATE inbox_mail SET status='failed' WHERE company_id=$1::text::bigint AND message_id='synthetic-gateway'").bind(&c.id).execute(&p).await.unwrap();
    let af = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(af.unread, 2);
    assert_eq!(af.events[0].code.as_deref(), Some("gateway_failed"));
    // Latest-50 display keeps a full unread count; explicit through marks older
    // events too, including those not present in the bounded response.
    sqlx::query("INSERT INTO inbox_notification_event(company_id,category,code) SELECT $1::text::bigint,'error','message_rejected' FROM generate_series(1,60)").bind(&c.id).execute(&p).await.unwrap();
    let af = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(af.events.len(), 50);
    assert_eq!(af.unread, 62);
    assert_eq!(
        store.read(&a.id, &c.id, &af.through).await.unwrap().unread,
        0
    );

    let mut args = serde_json::json!({"company_id":c.id,"unexpected":true})
        .as_object()
        .unwrap()
        .clone();
    assert!(faktura_server::inbox_notification::tool(
        p.clone(),
        &a.id,
        "inbox_notification_feed",
        std::mem::take(&mut args)
    )
    .await
    .is_err());
    assert!(inbox
        .upload(&a.id, &c.id, upload("forbidden"))
        .await
        .is_err());
    sqlx::query(
        "DELETE FROM membership WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint",
    )
    .bind(&c.id)
    .bind(&a.id)
    .execute(&p)
    .await
    .unwrap();
    assert!(store.feed(&a.id, &c.id).await.is_err());
    assert!(store.read(&a.id, &c.id, &before).await.is_err());
    assert_ne!(old.id, d.id);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn gateway_failure_after_rollback_uuid_dedup_then_success_retry(p: PgPool) {
    use faktura_application::inbox::{MailIntake, Settings};
    let auth = PgAuth::new(p.clone(), Some("synthetic-gateway-notify".into()));
    auth::bootstrap(
        &auth,
        "synthetic-gateway-notify",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, _) = auth::login(&auth, "owner@example.test", "Synthetic password 123!")
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
    let inbox = PgInbox(p.clone());
    let store = PgNotifications(p.clone());
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 1,
                new_documents: true,
                errors: true,
            },
        )
        .await
        .unwrap();
    let request = key();
    let input = |valid: bool| MailIntake {
        request_id: request.clone(),
        message_id: "<synthetic@example.test>".into(),
        sender: "sender@example.test".into(),
        subject: "private subject must not be retained".into(),
        attachments: vec![Upload {
            request_id: key(),
            filename: "safe.pdf".into(),
            base64: base64::engine::general_purpose::STANDARD.encode(if valid {
                b"%PDF-1.4\nsynthetic".as_slice()
            } else {
                b"not a PDF".as_slice()
            }),
        }],
    };
    assert!(inbox.mail(&a.id, &c.id, input(false)).await.is_err());
    assert_eq!(store.feed(&a.id, &c.id).await.unwrap().unread, 0);
    inbox
        .configure(
            &a.id,
            &c.id,
            Settings {
                revision: 0,
                email_enabled: true,
                extraction_provider: None,
            },
        )
        .await
        .unwrap();
    assert!(inbox.mail("999999", &c.id, input(false)).await.is_err());
    assert_eq!(store.feed(&a.id, &c.id).await.unwrap().unread, 0);
    assert!(inbox.mail(&a.id, &c.id, input(false)).await.is_err());
    assert!(inbox.mail(&a.id, &c.id, input(false)).await.is_err());
    let f = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(f.unread, 1);
    assert_eq!(f.events[0].code.as_deref(), Some("gateway_failed"));
    assert!(!serde_json::to_string(&f)
        .unwrap()
        .contains("private subject"));
    // Failure did not reserve the intake UUID/Message-ID or create a partial file.
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM inbox_mail")
            .fetch_one(&p)
            .await
            .unwrap(),
        0
    );
    inbox.mail(&a.id, &c.id, input(true)).await.unwrap();
    let f = store.feed(&a.id, &c.id).await.unwrap();
    assert_eq!(f.unread, 2);
    assert_eq!(
        f.events
            .iter()
            .filter(|e| e.code.as_deref() == Some("gateway_failed"))
            .count(),
        1
    );
}

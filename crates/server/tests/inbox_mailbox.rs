use faktura_application::{
    auth, company,
    inbox::{InboxStore, Search},
    inbox_mailbox::{Configure, MailboxStore},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{
    auth::PgAuth,
    company::PgCompany,
    inbox::PgInbox,
    inbox_mailbox::{Batch, MailboxConfig, Message, PgMailbox, Transport},
};
use sqlx::PgPool;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn raw(id: &str) -> Vec<u8> {
    format!("From: sender@example.test\r\nSubject: Synthetic IMAP\r\nMessage-ID: <{id}@example.test>\r\nContent-Type: multipart/mixed; boundary=bound\r\n\r\n--bound\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=original.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--bound--\r\n").into_bytes()
}
struct Fake {
    batch: Mutex<Option<Batch>>,
    calls: AtomicUsize,
    hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}
impl Fake {
    fn new(validity: u32, messages: Vec<Message>) -> Self {
        Self {
            batch: Mutex::new(Some(Batch {
                uid_validity: validity,
                messages,
            })),
            calls: AtomicUsize::new(0),
            hook: Mutex::new(None),
        }
    }
}
impl Transport for Fake {
    fn fetch(&self, _: MailboxConfig, _: Option<u32>, _: u32) -> company::Result<Batch> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(h) = self.hook.lock().unwrap().take() {
            h();
        }
        self.batch
            .lock()
            .unwrap()
            .take()
            .ok_or(company::CompanyError::Unavailable)
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn native_imap_opt_in_cursor_epoch_replay_rejection_and_mid_io_permissions(p: PgPool) {
    std::env::set_var(
        "INBOX_IMAP_PROVIDERS_JSON",
        r#"{"synthetic":{"host":"127.0.0.1","port":993,"username":"synthetic-user","password":"synthetic-password","mailbox":"INBOX"}}"#,
    );
    let auth = PgAuth::new(p.clone(), Some("synthetic-imap-bootstrap".into()));
    auth::bootstrap(
        &auth,
        "synthetic-imap-bootstrap",
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
    let other = company::create(
        &PgCompany(p.clone()),
        &a.id,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let fake = Arc::new(Fake::new(
        7,
        vec![
            Message {
                uid: 1,
                body: Some(raw("first")),
            },
            Message {
                uid: 2,
                body: Some(b"Subject: unsupported\r\n\r\nplain".to_vec()),
            },
            Message { uid: 3, body: None },
        ],
    ));
    let store = PgMailbox::with_transport(p.clone(), fake.clone());
    assert!(store.sync(&a.id, &c.id, &key()).await.is_err());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 0,
                enabled: true,
                automatic: false,
                provider: Some("synthetic".into()),
            },
        )
        .await
        .unwrap();
    let request = key();
    let done = store.sync(&a.id, &c.id, &request).await.unwrap();
    assert_eq!(done.messages, 3);
    assert_eq!(done.attachments, 1);
    assert_eq!(done.rejected, 2);
    assert_eq!(done.cursor, 3);
    assert_eq!(store.sync(&a.id, &c.id, &request).await.unwrap().cursor, 3);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        PgInbox(p.clone())
            .list(&a.id, &c.id, Search::default())
            .await
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        PgInbox(p.clone())
            .list(&a.id, &other.id, Search::default())
            .await
            .unwrap()
            .total,
        0
    );
    let reset = PgMailbox::with_transport(
        p.clone(),
        Arc::new(Fake::new(
            8,
            vec![Message {
                uid: 1,
                body: Some(raw("first")),
            }],
        )),
    );
    let done = reset.sync(&a.id, &c.id, &key()).await.unwrap();
    assert_eq!(done.uid_validity, Some(8));
    assert_eq!(done.cursor, 1);
    assert_eq!(
        PgInbox(p.clone())
            .list(&a.id, &c.id, Search::default())
            .await
            .unwrap()
            .total,
        1
    );
    // A concurrent replay must not release the first caller's unique claim.
    let parallel_fake = Arc::new(Fake::new(
        8,
        vec![Message {
            uid: 2,
            body: Some(raw("parallel")),
        }],
    ));
    let pool = p.clone();
    let actor = a.id.clone();
    let company = c.id.clone();
    let parallel_request = key();
    let nested_request = parallel_request.clone();
    *parallel_fake.hook.lock().unwrap() = Some(Box::new(move || {
        tokio::runtime::Handle::current().block_on(async {
            let second = PgMailbox::with_transport(pool, Arc::new(Fake::new(8, Vec::new())));
            assert!(second
                .sync(&actor, &company, &nested_request)
                .await
                .is_err());
        });
    }));
    let parallel = PgMailbox::with_transport(p.clone(), parallel_fake);
    assert_eq!(
        parallel
            .sync(&a.id, &c.id, &parallel_request)
            .await
            .unwrap()
            .cursor,
        2
    );
    // Revocation during external I/O is rechecked before any origin or cursor write.
    let fake = Arc::new(Fake::new(
        8,
        vec![Message {
            uid: 3,
            body: Some(raw("second")),
        }],
    ));
    let pool = p.clone();
    let company = c.id.clone();
    *fake.hook.lock().unwrap() = Some(Box::new(move || {
        tokio::runtime::Handle::current().block_on(async {
            sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
                .bind(company)
                .execute(&pool)
                .await
                .unwrap();
        });
    }));
    let revoked = PgMailbox::with_transport(p.clone(), fake);
    assert!(revoked.sync(&a.id, &c.id, &key()).await.is_err());
    assert_eq!(revoked.settings(&a.id, &c.id).await.unwrap().cursor, 2);
    assert_eq!(
        PgInbox(p.clone())
            .list(&a.id, &c.id, Search::default())
            .await
            .unwrap()
            .total,
        1
    );
    sqlx::query("UPDATE membership SET role='owner' WHERE company_id=$1::text::bigint")
        .bind(&c.id)
        .execute(&p)
        .await
        .unwrap();
    let toggled_fake = Arc::new(Fake::new(
        8,
        vec![Message {
            uid: 3,
            body: Some(raw("third")),
        }],
    ));
    let pool = p.clone();
    let actor = a.id.clone();
    let company = c.id.clone();
    *toggled_fake.hook.lock().unwrap() = Some(Box::new(move || {
        tokio::runtime::Handle::current().block_on(async {
            PgMailbox::new(pool)
                .configure(
                    &actor,
                    &company,
                    Configure {
                        revision: 1,
                        enabled: false,
                        automatic: false,
                        provider: Some("synthetic".into()),
                    },
                )
                .await
                .unwrap();
        });
    }));
    let toggled = PgMailbox::with_transport(p.clone(), toggled_fake);
    assert!(toggled.sync(&a.id, &c.id, &key()).await.is_err());
    assert_eq!(toggled.settings(&a.id, &c.id).await.unwrap().cursor, 2);
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 2,
                enabled: true,
                automatic: false,
                provider: Some("synthetic".into()),
            },
        )
        .await
        .unwrap();
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM inbox_mailbox_settings_audit WHERE company_id=$1::text::bigint",
    )
    .bind(&c.id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(audit, 3);
    assert!(store
        .history(&a.id, &c.id)
        .await
        .unwrap()
        .iter()
        .any(|r| r.status == "failed"));
    let settings = store.settings(&a.id, &c.id).await.unwrap();
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: settings.revision,
                enabled: false,
                automatic: false,
                provider: settings.provider,
            },
        )
        .await
        .unwrap();
    assert!(store.sync(&a.id, &c.id, &key()).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn scheduled_imap_explicit_opt_in_concurrent_workers_and_actor_revocation(p: PgPool) {
    std::env::set_var(
        "INBOX_IMAP_PROVIDERS_JSON",
        r#"{"synthetic":{"host":"127.0.0.1","port":993,"username":"synthetic-user","password":"synthetic-password","mailbox":"INBOX"}}"#,
    );
    let auth = PgAuth::new(p.clone(), Some("synthetic-scheduled-imap".into()));
    auth::bootstrap(
        &auth,
        "synthetic-scheduled-imap",
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
    let fake = Arc::new(Fake::new(
        11,
        vec![Message {
            uid: 1,
            body: Some(raw("scheduled")),
        }],
    ));
    let store = PgMailbox::with_transport(p.clone(), fake.clone());
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 0,
                enabled: true,
                automatic: false,
                provider: Some("synthetic".into()),
            },
        )
        .await
        .unwrap();
    assert!(!store.run_scheduled().await.unwrap());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 1,
                enabled: true,
                automatic: true,
                provider: Some("synthetic".into()),
            },
        )
        .await
        .unwrap();
    let (left, right) = tokio::join!(store.run_scheduled(), store.run_scheduled());
    assert_eq!(usize::from(left.unwrap()) + usize::from(right.unwrap()), 1);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let settings = store.settings(&a.id, &c.id).await.unwrap();
    assert!(settings.automatic);
    assert!(settings.next_run_at.is_some());
    assert_eq!(settings.cursor, 1);
    sqlx::query("UPDATE inbox_mailbox_settings SET next_run_at=now()-interval '1 second' WHERE company_id=$1::text::bigint").bind(&c.id).execute(&p).await.unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c.id)
        .execute(&p)
        .await
        .unwrap();
    assert!(store.run_scheduled().await.unwrap());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    assert!(store
        .history(&a.id, &c.id)
        .await
        .unwrap()
        .iter()
        .any(|r| r.error.as_deref() == Some("forbidden")));
    sqlx::query("UPDATE membership SET role='owner' WHERE company_id=$1::text::bigint")
        .bind(&c.id)
        .execute(&p)
        .await
        .unwrap();
    store
        .configure(
            &a.id,
            &c.id,
            Configure {
                revision: 2,
                enabled: true,
                automatic: false,
                provider: Some("synthetic".into()),
            },
        )
        .await
        .unwrap();
    assert!(!store.run_scheduled().await.unwrap());
}

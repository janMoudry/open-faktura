use super::*;
use faktura_application::bank_connection::{self as bc, ConnectionStore};
use faktura_infrastructure::bank_connection::{
    transport::{Endpoint, Transport},
    PgConnection,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
struct Fake {
    calls: AtomicUsize,
    mode: AtomicUsize,
}
#[async_trait::async_trait]
impl Transport for Fake {
    async fn fetch(
        &self,
        _: &Endpoint,
        token: &str,
        iban: &str,
        _: &str,
        to: &str,
    ) -> Result<Vec<u8>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(token, "a".repeat(64));
        let mode = self.mode.load(Ordering::SeqCst);
        if mode == 1 {
            return Err("connection_auth".into());
        }
        if mode == 4 {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        let mut v: serde_json::Value =
            serde_json::from_str(include_str!("../../../../tests/fixtures/bank/fio.json")).unwrap();
        v["accountStatement"]["info"]["iban"] = serde_json::json!(if mode == 2 {
            "CZ0000000000000000000000"
        } else {
            iban
        });
        v["accountStatement"]["transactionList"]["transaction"][0]["column0"]["value"] =
            serde_json::json!(to);
        v["accountStatement"]["transactionList"]["transaction"][0]["column1"]["value"] =
            serde_json::json!("100.01");
        if mode == 3 {
            v["accountStatement"]["transactionList"] = serde_json::Value::Null;
        }
        if mode == 5 {
            v["accountStatement"]["transactionList"]["transaction"][0]["column1"]["value"] =
                serde_json::json!("200.02");
        }
        Ok(serde_json::to_vec(&v).unwrap())
    }
}
fn command(ac: &Account, revision: i64, action: bc::Action) -> bc::Command {
    bc::Command {
        account_id: ac.id.clone(),
        revision,
        request_id: key(),
        action,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL; run init-secrets first"]
async fn encrypted_scope_test_sync_replay_disconnect(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    let fake = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        mode: AtomicUsize::new(0),
    });
    let s = PgConnection {
        pool: p.clone(),
        transport: fake.clone(),
    };
    assert!(!s.status(&a, &c, &ac.id).await.unwrap().configured);
    assert!(s.status(&a, &other, &ac.id).await.is_err());
    let save = command(
        &ac,
        0,
        bc::Action::Save {
            provider: "fio".into(),
            secret: Some("a".repeat(64)),
        },
    );
    let v = s.execute(&a, &c, save).await.unwrap();
    assert!(v.configured);
    assert_eq!(v.revision, 1);
    let encrypted: Vec<u8> = sqlx::query_scalar("SELECT secret FROM bank_connection")
        .fetch_one(&p)
        .await
        .unwrap();
    assert!(!encrypted
        .windows(64)
        .any(|v| v == "a".repeat(64).as_bytes()));
    assert!(!serde_json::to_string(&v).unwrap().contains(&"a".repeat(64)));
    let audit: String =
        sqlx::query_scalar("SELECT row_to_json(r)::text FROM bank_connection_run r")
            .fetch_one(&p)
            .await
            .unwrap();
    assert!(!audit.contains(&"a".repeat(64)));
    assert!(s
        .execute(&a, &c, command(&ac, 0, bc::Action::Disconnect))
        .await
        .is_err());
    let test = s
        .execute(&a, &c, command(&ac, 1, bc::Action::Test))
        .await
        .unwrap();
    assert_eq!(test.last_run.unwrap().status, "success");
    assert_eq!(
        bank.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
    let cmd = command(&ac, 1, bc::Action::Sync);
    let repeat = serde_json::to_value(&cmd).unwrap();
    let synced = s.execute(&a, &c, cmd).await.unwrap();
    assert_eq!(synced.last_run.unwrap().imported, 1);
    assert!(synced.last_sync_at.is_some());
    let count = fake.calls.load(Ordering::SeqCst);
    s.execute(&a, &c, serde_json::from_value(repeat).unwrap())
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), count);
    let duplicate = s
        .execute(&a, &c, command(&ac, 1, bc::Action::Sync))
        .await
        .unwrap();
    assert_eq!(duplicate.last_run.unwrap().duplicates, 1);
    fake.mode.store(5, Ordering::SeqCst);
    assert_eq!(
        s.execute(&a, &c, command(&ac, 1, bc::Action::Sync))
            .await
            .unwrap()
            .last_run
            .unwrap()
            .error_code
            .as_deref(),
        Some("connection_conflict")
    );
    for (mode, code) in [(1, "connection_auth"), (2, "connection_account")] {
        fake.mode.store(mode, Ordering::SeqCst);
        let v = s
            .execute(&a, &c, command(&ac, 1, bc::Action::Sync))
            .await
            .unwrap();
        assert_eq!(v.last_run.unwrap().error_code.as_deref(), Some(code));
    }
    fake.mode.store(3, Ordering::SeqCst);
    assert_eq!(
        s.execute(&a, &c, command(&ac, 1, bc::Action::Sync))
            .await
            .unwrap()
            .last_run
            .unwrap()
            .imported,
        0
    );
    assert_eq!(
        bank.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
    let disconnected = s
        .execute(&a, &c, command(&ac, 1, bc::Action::Disconnect))
        .await
        .unwrap();
    assert!(!disconnected.configured);
    let erased: Option<Vec<u8>> = sqlx::query_scalar("SELECT secret FROM bank_connection")
        .fetch_one(&p)
        .await
        .unwrap();
    assert!(erased.is_none());
    assert_eq!(
        bank.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
    assert!(s
        .execute(&a, &c, command(&ac, 2, bc::Action::Sync))
        .await
        .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL; run init-secrets first"]
async fn concurrent_run_blocks_configuration_and_viewer(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let ac = account(&PgBank(p.clone()), &a, &c).await;
    let fake = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        mode: AtomicUsize::new(4),
    });
    let s = PgConnection {
        pool: p.clone(),
        transport: fake.clone(),
    };
    s.execute(
        &a,
        &c,
        command(
            &ac,
            0,
            bc::Action::Save {
                provider: "fio".into(),
                secret: Some("a".repeat(64)),
            },
        ),
    )
    .await
    .unwrap();
    let (first, second) = tokio::join!(
        s.execute(&a, &c, command(&ac, 1, bc::Action::Sync)),
        async {
            while fake.calls.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
            s.execute(&a, &c, command(&ac, 1, bc::Action::Disconnect))
                .await
        }
    );
    assert!(first.is_ok());
    assert!(second.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1 AND user_id=$2")
        .bind(c.parse::<i64>().unwrap())
        .bind(a.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    assert!(s.status(&a, &c, &ac.id).await.is_ok());
    assert!(s
        .execute(&a, &c, command(&ac, 1, bc::Action::Test))
        .await
        .is_err());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade032_preserves_movements_without_enabling_connection(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140033)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let bank = PgBank(p.clone());
    let ac = account(&bank, &a, &c).await;
    movement(&bank, &a, &c, &ac, "100").await;
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let s = PgConnection::new(p.clone());
    assert!(!s.status(&a, &c, &ac.id).await.unwrap().configured);
    assert_eq!(
        bank.movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL; run init-secrets first"]
async fn scheduled_sync_claim_opt_in_revoke_and_disable(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let ac = account(&PgBank(p.clone()), &a, &c).await;
    let fake = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        mode: AtomicUsize::new(4),
    });
    let s = PgConnection {
        pool: p.clone(),
        transport: fake.clone(),
    };
    s.execute(
        &a,
        &c,
        command(
            &ac,
            0,
            bc::Action::Save {
                provider: "fio".into(),
                secret: Some("a".repeat(64)),
            },
        ),
    )
    .await
    .unwrap();
    assert!(!s.process_one().await.unwrap());
    assert!(!s.status(&a, &c, &ac.id).await.unwrap().schedule_enabled);
    assert!(s
        .execute(
            &a,
            &c,
            command(
                &ac,
                1,
                bc::Action::Schedule {
                    enabled: true,
                    interval_minutes: 1
                }
            )
        )
        .await
        .is_err());
    let v = s
        .execute(
            &a,
            &c,
            command(
                &ac,
                1,
                bc::Action::Schedule {
                    enabled: true,
                    interval_minutes: 15,
                },
            ),
        )
        .await
        .unwrap();
    assert!(v.schedule_enabled);
    assert!(v.next_run_at.is_some());
    assert_eq!(v.revision, 2);
    assert!(!s.process_one().await.unwrap());
    sqlx::query("UPDATE bank_connection SET next_run_at=now()-interval '1 hour'")
        .execute(&p)
        .await
        .unwrap();
    let (x, y) = tokio::join!(s.process_one(), s.process_one());
    assert_ne!(x.unwrap(), y.unwrap());
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
    let v = s.status(&a, &c, &ac.id).await.unwrap();
    assert_eq!(v.last_run.unwrap().imported, 1);
    assert!(v.last_sync_at.is_some());
    // A provider failure schedules the next bounded attempt instead of busy-looping.
    fake.mode.store(1, Ordering::SeqCst);
    sqlx::query("UPDATE bank_connection SET next_run_at=now()-interval '1 minute'")
        .execute(&p)
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    assert!(!s.process_one().await.unwrap());
    assert_eq!(
        s.status(&a, &c, &ac.id)
            .await
            .unwrap()
            .last_run
            .unwrap()
            .error_code
            .as_deref(),
        Some("connection_auth")
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    sqlx::query("UPDATE bank_connection SET next_run_at=now()-interval '1 minute'")
        .execute(&p)
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    let v = s.status(&a, &c, &ac.id).await.unwrap();
    assert!(!v.schedule_enabled);
    assert!(v.next_run_at.is_none());
    assert_eq!(v.schedule_error.as_deref(), Some("connection_permission"));
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2);
    sqlx::query("UPDATE membership SET role='owner' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(!s.process_one().await.unwrap());
    let v = s
        .execute(
            &a,
            &c,
            command(
                &ac,
                v.revision,
                bc::Action::Schedule {
                    enabled: true,
                    interval_minutes: 60,
                },
            ),
        )
        .await
        .unwrap();
    let v = s
        .execute(
            &a,
            &c,
            command(
                &ac,
                v.revision,
                bc::Action::Schedule {
                    enabled: false,
                    interval_minutes: 60,
                },
            ),
        )
        .await
        .unwrap();
    assert!(!v.schedule_enabled);
    assert!(!s.process_one().await.unwrap());
    let v = s
        .execute(
            &a,
            &c,
            command(
                &ac,
                v.revision,
                bc::Action::Schedule {
                    enabled: true,
                    interval_minutes: 60,
                },
            ),
        )
        .await
        .unwrap();
    let v = s
        .execute(
            &a,
            &c,
            command(
                &ac,
                v.revision,
                bc::Action::Save {
                    provider: "fio".into(),
                    secret: None,
                },
            ),
        )
        .await
        .unwrap();
    assert!(!v.schedule_enabled);
    assert!(v.next_run_at.is_none());
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade033_preserves_credentials_and_disables_schedules(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140034)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let ac = account(&PgBank(p.clone()), &a, &c).await;
    // Opaque synthetic ciphertext fixture: this test verifies preservation, never decrypts or calls the bank.
    let bytes = vec![17u8; 80];
    sqlx::query("INSERT INTO bank_connection(company_id,account_id,provider,secret,revision) VALUES($1::text::bigint,$2::text::bigint,'fio',$3,4)").bind(&c).bind(&ac.id).bind(&bytes).execute(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let s = PgConnection::new(p.clone());
    let status = s.status(&a, &c, &ac.id).await.unwrap();
    assert!(status.configured);
    assert!(!status.schedule_enabled);
    assert!(status.next_run_at.is_none());
    assert_eq!(status.revision, 4);
    let stored: Vec<u8> = sqlx::query_scalar("SELECT secret FROM bank_connection")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(stored, bytes);
    assert!(!s.process_one().await.unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL; run init-secrets first"]
async fn scheduled_sync_rechecks_rights_after_network(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let ac = account(&PgBank(p.clone()), &a, &c).await;
    let fake = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        mode: AtomicUsize::new(4),
    });
    let s = PgConnection {
        pool: p.clone(),
        transport: fake.clone(),
    };
    s.execute(
        &a,
        &c,
        command(
            &ac,
            0,
            bc::Action::Save {
                provider: "fio".into(),
                secret: Some("a".repeat(64)),
            },
        ),
    )
    .await
    .unwrap();
    s.execute(
        &a,
        &c,
        command(
            &ac,
            1,
            bc::Action::Schedule {
                enabled: true,
                interval_minutes: 15,
            },
        ),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE bank_connection SET next_run_at=now()-interval '1 minute'")
        .execute(&p)
        .await
        .unwrap();
    let (result, ()) = tokio::join!(s.process_one(), async {
        while fake.calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
        sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
            .bind(&c)
            .execute(&p)
            .await
            .unwrap();
    });
    assert!(result.unwrap());
    let status = s.status(&a, &c, &ac.id).await.unwrap();
    assert!(!status.schedule_enabled);
    assert_eq!(
        status.schedule_error.as_deref(),
        Some("connection_permission")
    );
    assert_eq!(
        PgBank(p.clone())
            .movements(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
}

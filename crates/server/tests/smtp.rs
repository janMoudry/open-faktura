use faktura_application::{
    auth,
    company::CompanyError as Error,
    smtp::{self, Save, Settings, SmtpStore, Test},
};
use faktura_infrastructure::{auth::PgAuth, smtp::PgSmtp};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
async fn setup(pool: &PgPool) -> (String, String) {
    let store = PgAuth::new(
        pool.clone(),
        Some("synthetic-bootstrap-only-test-12345678".into()),
    );
    auth::bootstrap(
        &store,
        "synthetic-bootstrap-only-test-12345678",
        "admin@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, t) = auth::login(&store, "admin@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    (a.id, t)
}
fn config(port: u16) -> Settings {
    Settings {
        revision: 0,
        mode: "custom".into(),
        host: "127.0.0.1".into(),
        port,
        security: "plain".into(),
        sender: "sender@example.test".into(),
        username: "".into(),
    }
}
fn command(revision: i64) -> Test {
    Test {
        request_id: KEY.into(),
        revision,
        recipient: "recipient@example.test".into(),
        language: "cs".into(),
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
#[ignore = "requires isolated PostgreSQL and init-secrets"]
async fn secrets_roles_revisions_and_live_config(pool: PgPool) {
    let (a, _) = setup(&pool).await;
    let store = PgSmtp {
        pool: pool.clone(),
        fallback: None,
    };
    let other:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('other@example.test','unused') RETURNING id").fetch_one(&pool).await.unwrap();
    assert!(matches!(
        store.get(&other.to_string()).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        smtp::save(
            &store,
            &other.to_string(),
            Save {
                settings: config(2525),
                password: None
            }
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        smtp::test(&store, &other.to_string(), command(0)).await,
        Err(Error::Forbidden)
    ));
    let state = store.get(&a).await.unwrap();
    assert_eq!(state.settings.mode, "environment");
    assert!(!state.configured);
    let mut cfg = config(2525);
    cfg.username = "user".into();
    let state = smtp::save(
        &store,
        &a,
        Save {
            settings: cfg.clone(),
            password: Some("synthetic-smtp-secret".into()),
        },
    )
    .await
    .unwrap();
    assert!(state.password_set);
    let exposed = serde_json::to_string(&state).unwrap();
    assert!(!exposed.contains("synthetic-smtp-secret"));
    let encrypted: Vec<u8> = sqlx::query_scalar("SELECT password FROM smtp_settings")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!encrypted.windows(21).any(|v| v == b"synthetic-smtp-secret"));
    cfg.revision = state.settings.revision;
    smtp::save(
        &store,
        &a,
        Save {
            settings: cfg.clone(),
            password: None,
        },
    )
    .await
    .unwrap();
    let same: Vec<u8> = sqlx::query_scalar("SELECT password FROM smtp_settings")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(same, encrypted);
    let mut corrupt = encrypted.clone();
    corrupt[13] ^= 1;
    sqlx::query("UPDATE smtp_settings SET password=$1")
        .bind(corrupt)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        faktura_infrastructure::smtp::resolve(
            &pool,
            Some(std::sync::Arc::new(
                faktura_infrastructure::delivery::MailConfig::local_capture(2525)
            ))
        )
        .await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE smtp_settings SET password=$1")
        .bind(&encrypted)
        .execute(&pool)
        .await
        .unwrap();

    assert!(matches!(
        smtp::save(
            &store,
            &a,
            Save {
                settings: cfg.clone(),
                password: None
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    cfg.revision += 1;
    cfg.username.clear();
    let state = smtp::save(
        &store,
        &a,
        Save {
            settings: cfg.clone(),
            password: None,
        },
    )
    .await
    .unwrap();
    assert!(!state.password_set);
    let mail = faktura_infrastructure::smtp::resolve(&pool, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(mail.sender, "sender@example.test");
    cfg.revision = state.settings.revision;
    cfg.mode = "disabled".into();
    smtp::save(
        &store,
        &a,
        Save {
            settings: cfg,
            password: None,
        },
    )
    .await
    .unwrap();
    assert!(faktura_infrastructure::smtp::resolve(
        &pool,
        Some(std::sync::Arc::new(mail.as_ref().clone()))
    )
    .await
    .unwrap()
    .is_none());
    let mut bad = config(587);
    bad.host = "remote.example.test".into();
    bad.revision = 4;
    assert!(matches!(
        smtp::save(
            &store,
            &a,
            Save {
                settings: bad,
                password: None
            }
        )
        .await,
        Err(Error::Invalid(_))
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn test_mail_retry_failure_and_recovery(pool: PgPool) {
    let (a, _) = setup(&pool).await;
    let store = PgSmtp {
        pool: pool.clone(),
        fallback: None,
    };
    let (port, capture) = smtp(false).await;
    smtp::save(
        &store,
        &a,
        Save {
            settings: config(port),
            password: None,
        },
    )
    .await
    .unwrap();
    let (one, two) = tokio::join!(
        smtp::test(&store, &a, command(1)),
        smtp::test(&store, &a, command(1))
    );
    assert_eq!(one.unwrap().request_id, two.unwrap().request_id);
    let message = capture.await.unwrap();
    assert!(message.contains("recipient@example.test"));
    let replay = smtp::test(&store, &a, command(1)).await.unwrap();
    assert_eq!(replay.status, "sent");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM smtp_test")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let mut changed = command(1);
    changed.recipient = "different@example.test".into();
    assert!(matches!(
        smtp::test(&store, &a, changed).await,
        Err(Error::Conflict)
    ));
    let (port, capture) = smtp(true).await;
    let mut cfg = config(port);
    cfg.revision = 1;
    smtp::save(
        &store,
        &a,
        Save {
            settings: cfg,
            password: None,
        },
    )
    .await
    .unwrap();
    let mut cmd = command(2);
    cmd.request_id = "00000000-0000-4000-8000-000000000002".into();
    assert_eq!(smtp::test(&store, &a, cmd).await.unwrap().status, "failed");
    capture.await.unwrap();
    sqlx::query("UPDATE smtp_test SET status='sending',created_at=now()-interval '3 minutes' WHERE request_id=$1::text::uuid").bind(KEY).execute(&pool).await.unwrap();
    assert_eq!(
        smtp::test(&store, &a, command(1)).await.unwrap().status,
        "uncertain"
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn rest_authorization_and_contract(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (_, token) = setup(&pool).await;
    let app = faktura_server::smtp::router(
        PgSmtp {
            pool,
            fallback: None,
        },
        vec!["http://localhost:5173".into()],
        false,
    );
    let cookie = format!("faktura_session={token}");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/installation/smtp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/installation/smtp")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = to_bytes(response.into_body(), 16384).await.unwrap();
    let state: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(state.get("password").is_none());
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/installation/smtp")
                .header("cookie", cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&Save {
                        settings: config(2525),
                        password: None,
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn mcp_and_rest_save_same_settings(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt};
    use tower::ServiceExt as _;
    let (_, token) = setup(&pool).await;
    let app = faktura_server::smtp::router(
        PgSmtp {
            pool: pool.clone(),
            fallback: None,
        },
        vec!["http://localhost:5173".into()],
        false,
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/installation/smtp")
                .header("cookie", format!("faktura_session={token}"))
                .header("origin", "http://localhost:5173")
                .header("x-faktura-request", "1")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&Save {
                        settings: config(2525),
                        password: None,
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let rest: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap();
    let server = faktura_server::mcp::CompanyMcp::new(pool.clone(), token);
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
    let result = client
        .call_tool(CallToolRequestParams::new("smtp_get"))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let mcp: serde_json::Value =
        serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(mcp, rest);
    let mut cfg = config(2525);
    cfg.revision = 1;
    cfg.mode = "disabled".into();
    let value = serde_json::to_value(Save {
        settings: cfg,
        password: None,
    })
    .unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("smtp_save")
                .with_arguments(value.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    assert!(faktura_infrastructure::smtp::resolve(&pool, None)
        .await
        .unwrap()
        .is_none());
    client.cancel().await.unwrap();
    task.await.unwrap();
}

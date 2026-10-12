use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use faktura_application::auth::{self, AuthError, AuthStore};
use faktura_infrastructure::auth::PgAuth;
use sqlx::PgPool;
use tower::ServiceExt;
const SECRET: &str = "synthetic-bootstrap-secret-for-isolated-tests-only";
const PASSWORD: &str = "Synthetic password 123!";
fn store(pool: &PgPool) -> PgAuth {
    PgAuth::new(pool.clone(), Some(SECRET.into()))
}
fn app(pool: &PgPool, secure: bool) -> Router {
    faktura_server::auth::router(
        pool.clone(),
        Some(SECRET.into()),
        vec!["https://app.example.test".into()],
        secure,
    )
}
async fn call(
    app: Router,
    path: &str,
    body: Option<serde_json::Value>,
    cookie: Option<&str>,
    origin: Option<&str>,
) -> axum::response::Response {
    let mut req = Request::builder().uri(path);
    if let Some(cookie) = cookie {
        req = req.header("cookie", cookie);
    }
    if let Some(origin) = origin {
        req = req
            .header("origin", origin)
            .header("x-faktura-request", "1");
    }
    let body = if let Some(body) = body {
        req = req
            .method("POST")
            .header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    app.oneshot(req.body(body).unwrap()).await.unwrap()
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn bootstrap_is_atomic_and_cannot_reopen(pool: PgPool) {
    let s = store(&pool);
    assert!(s.setup_required().await.unwrap());
    assert_eq!(
        auth::bootstrap(&s, "wrong", "admin@example.test", PASSWORD).await,
        Err(AuthError::Unauthorized)
    );
    assert_eq!(
        auth::bootstrap(&s, SECRET, "admin@example.test", "short").await,
        Err(AuthError::InvalidInput)
    );
    let (a, b) = tokio::join!(
        auth::bootstrap(&s, SECRET, "admin@example.test", PASSWORD),
        auth::bootstrap(&s, SECRET, "other@example.test", PASSWORD)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(a == Err(AuthError::AlreadyConfigured) || b == Err(AuthError::AlreadyConfigured));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM app_user")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(!PgAuth::new(pool.clone(), None)
        .setup_required()
        .await
        .unwrap());
    assert_eq!(
        auth::bootstrap(&s, SECRET, "again@example.test", PASSWORD).await,
        Err(AuthError::AlreadyConfigured)
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn sessions_expire_revoke_and_recovery_preserves_setup(pool: PgPool) {
    let s = store(&pool);
    auth::bootstrap(&s, SECRET, "ADMIN@EXAMPLE.TEST", PASSWORD)
        .await
        .unwrap();
    assert_eq!(
        auth::login(&s, "admin@example.test", "wrong").await,
        Err(AuthError::Unauthorized)
    );
    let (_, token) = auth::login(&s, "admin@example.test", PASSWORD)
        .await
        .unwrap();
    let restarted = PgAuth::new(pool.clone(), None);
    assert_eq!(
        restarted.session(&token).await.unwrap().email,
        "admin@example.test"
    );
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM auth_session LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, token);
    s.logout(&token).await.unwrap();
    assert_eq!(s.session(&token).await, Err(AuthError::Unauthorized));
    let (_, expired) = auth::login(&s, "admin@example.test", PASSWORD)
        .await
        .unwrap();
    sqlx::query("UPDATE auth_session SET expires_at=now()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(s.session(&expired).await, Err(AuthError::Unauthorized));
    let (_, revoked) = auth::login(&s, "admin@example.test", PASSWORD)
        .await
        .unwrap();
    auth::recover(&s, "admin@example.test", "A replacement password 456!")
        .await
        .unwrap();
    assert_eq!(s.session(&revoked).await, Err(AuthError::Unauthorized));
    assert_eq!(
        auth::login(&s, "admin@example.test", PASSWORD).await,
        Err(AuthError::Unauthorized)
    );
    assert!(
        auth::login(&s, "admin@example.test", "A replacement password 456!")
            .await
            .is_ok()
    );
    assert!(!s.setup_required().await.unwrap());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn http_cookie_csrf_and_private_responses(pool: PgPool) {
    let router = app(&pool, true);
    let payload =
        serde_json::json!({"secret":SECRET,"email":"admin@example.test","password":PASSWORD});
    for origin in [None, Some("https://evil.example.test")] {
        assert_eq!(
            call(
                router.clone(),
                "/api/auth/setup",
                Some(payload.clone()),
                None,
                origin
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        call(
            router.clone(),
            "/api/auth/setup",
            Some(payload),
            None,
            Some("https://app.example.test")
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    let login = call(
        router.clone(),
        "/api/auth/login",
        Some(serde_json::json!({"email":"admin@example.test","password":PASSWORD})),
        None,
        Some("https://app.example.test"),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    assert_eq!(login.headers()["cache-control"], "no-store");
    let cookie = login.headers()["set-cookie"].to_str().unwrap().to_owned();
    assert!(cookie.starts_with("__Host-faktura_session="));
    for attribute in [
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Path=/",
        "Max-Age=43200",
    ] {
        assert!(cookie.contains(attribute));
    }
    let body =
        String::from_utf8(to_bytes(login.into_body(), 8192).await.unwrap().to_vec()).unwrap();
    assert!(!body.contains("password") && !body.contains("token"));
    let cookie = cookie.split(';').next().unwrap();
    assert_eq!(
        call(
            router.clone(),
            "/api/auth/session",
            None,
            Some(cookie),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        call(
            router.clone(),
            "/api/auth/logout",
            Some(serde_json::json!({})),
            Some(cookie),
            None
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            router.clone(),
            "/api/auth/logout",
            Some(serde_json::json!({})),
            Some(cookie),
            Some("https://app.example.test")
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(router, "/api/auth/session", None, Some(cookie), None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn limits_are_shared_across_instances(pool: PgPool) {
    let s = store(&pool);
    for _ in 0..10 {
        assert_eq!(
            auth::login(&s, "missing@example.test", PASSWORD).await,
            Err(AuthError::Unauthorized)
        );
    }
    assert_eq!(
        auth::login(&store(&pool), "missing@example.test", PASSWORD).await,
        Err(AuthError::RateLimited)
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn database_failure_is_not_setup_required(pool: PgPool) {
    pool.close().await;
    let response = call(app(&pool, false), "/api/auth/setup", None, None, None).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL databases"]
async fn account_changes_protect_credentials_and_revoke_other_sessions(pool: PgPool) {
    let s = store(&pool);
    auth::bootstrap(&s, SECRET, "admin@example.test", PASSWORD)
        .await
        .unwrap();
    let (_, token) = auth::login(&s, "admin@example.test", PASSWORD)
        .await
        .unwrap();
    let (_, other) = auth::login(&s, "admin@example.test", PASSWORD)
        .await
        .unwrap();
    let cookie = format!("faktura_session={token}");
    let body = serde_json::json!({"name":"Jan Test", "email":"admin@example.test"});
    assert_eq!(
        call(
            app(&pool, false),
            "/api/auth/account",
            Some(body.clone()),
            None,
            Some("https://app.example.test")
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app(&pool, false),
            "/api/auth/account",
            Some(body.clone()),
            Some(&cookie),
            Some("https://evil.example.test")
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = call(
        app(&pool, false),
        "/api/auth/account",
        Some(body),
        Some(&cookie),
        Some("https://app.example.test"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(s.session(&other).await.unwrap().name, "Jan Test");
    assert_eq!(
        auth::update_account(&s, &token, "Jan", "new@example.test", "wrong", None).await,
        Err(AuthError::InvalidPassword)
    );
    assert_eq!(
        auth::update_account(&s, &token, "Jan", "bad", PASSWORD, None).await,
        Err(AuthError::InvalidInput)
    );
    assert_eq!(
        auth::update_account(
            &s,
            &token,
            "Jan",
            "admin@example.test",
            PASSWORD,
            Some("short")
        )
        .await,
        Err(AuthError::InvalidInput)
    );
    sqlx::query("INSERT INTO app_user(email,password_hash) SELECT 'taken@example.test',password_hash FROM app_user LIMIT 1").execute(&pool).await.unwrap();
    assert_eq!(
        auth::update_account(&s, &token, "Jan", "taken@example.test", PASSWORD, None).await,
        Err(AuthError::EmailTaken)
    );
    assert_eq!(s.session(&token).await.unwrap().email, "admin@example.test");
    let updated = auth::update_account(
        &s,
        &token,
        "  Jan Test  ",
        "NEW@EXAMPLE.TEST",
        PASSWORD,
        Some("Changed password 123!"),
    )
    .await
    .unwrap();
    assert_eq!(updated.name, "Jan Test");
    assert_eq!(updated.email, "new@example.test");
    assert_eq!(s.session(&token).await.unwrap(), updated);
    assert_eq!(s.session(&other).await, Err(AuthError::Unauthorized));
    assert_eq!(
        auth::login(&s, "admin@example.test", PASSWORD).await,
        Err(AuthError::Unauthorized)
    );
    assert_eq!(
        auth::login(&s, "new@example.test", PASSWORD).await,
        Err(AuthError::Unauthorized)
    );
    assert!(auth::login(&s, "new@example.test", "Changed password 123!")
        .await
        .is_ok());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM installation_audit WHERE action='account_credentials_changed'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

use faktura_application::{
    auth,
    preferences::{PreferenceStore, Preferences, Theme},
};
use faktura_infrastructure::{auth::PgAuth, preferences::PgPreferences};
use sqlx::PgPool;
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn personal_theme_revision_replay_scope_and_rest(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    let auth = PgAuth::new(
        p.clone(),
        Some("synthetic-preferences-bootstrap-only".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-preferences-bootstrap-only",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, token) = auth::login(&auth, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let store = PgPreferences(p.clone());
    let initial = store.get(&a.id).await.unwrap();
    assert_eq!(initial.revision, 0);
    assert_eq!(initial.theme, Theme::System);
    let dark = store
        .set(
            &a.id,
            Preferences {
                revision: 0,
                theme: Theme::Dark,
            },
        )
        .await
        .unwrap();
    assert_eq!(dark.revision, 1);
    assert_eq!(
        store
            .set(
                &a.id,
                Preferences {
                    revision: 0,
                    theme: Theme::Dark
                }
            )
            .await
            .unwrap()
            .revision,
        1
    );
    assert!(store
        .set(
            &a.id,
            Preferences {
                revision: 0,
                theme: Theme::Light
            }
        )
        .await
        .is_err());
    let outsider:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES('another@example.test','no-login',false) RETURNING id").fetch_one(&p).await.unwrap();
    assert_eq!(
        store.get(&outsider.to_string()).await.unwrap().theme,
        Theme::System
    );
    let app =
        faktura_server::preferences::router(p.clone(), vec!["http://localhost:5173".into()], false);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/preferences")
                .header("cookie", format!("faktura_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let v: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 10000).await.unwrap()).unwrap();
    assert_eq!(v["theme"], "dark");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/preferences")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}

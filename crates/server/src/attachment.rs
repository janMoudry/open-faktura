use crate::{
    auth::{self, AuthState},
    company::Failure,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use faktura_application::attachment::{self, AttachmentStore, File, Upload};
use faktura_infrastructure::{attachment::PgAttachment, auth::PgAuth};
use sqlx::PgPool;
use std::sync::Arc;
#[derive(Clone)]
struct S {
    store: PgAttachment,
    auth: AuthState,
}
pub fn router(pool: PgPool, origins: Vec<String>, secure: bool) -> Router {
    Router::new()
        .route(
            "/api/companies/{c}/attachments/{kind}/{doc}",
            get(list).post(upload),
        )
        .route(
            "/api/companies/{c}/attachments/{kind}/{doc}/{file}",
            get(download).delete(delete),
        )
        .route(
            "/api/companies/{c}/attachments/{kind}/{doc}/deliveries/{delivery}/{file}",
            get(historical),
        )
        .layer(DefaultBodyLimit::max(2800000))
        .layer(axum::middleware::map_response(
            |mut r: Response| async move {
                r.headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                r
            },
        ))
        .with_state(S {
            store: PgAttachment(pool.clone()),
            auth: AuthState {
                store: Arc::new(PgAuth::new(pool, None)),
                origins,
                secure,
            },
        })
}
async fn actor(s: &S, h: &HeaderMap, write: bool) -> Result<String, auth::Failure> {
    if write {
        auth::same_origin(&s.auth, h)?;
    }
    Ok(s.auth
        .store
        .session(&auth::token(h, s.auth.secure))
        .await?
        .id)
}
fn response<T: serde::Serialize>(r: attachment::Result<T>) -> Response {
    match r {
        Ok(v) => Json(v).into_response(),
        Err(e) => Failure(e).into_response(),
    }
}
async fn list(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, d)): Path<(String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(s.store.list(&a, &c, &k, &d).await)
}
async fn upload(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, d)): Path<(String, String, String)>,
    Json(cmd): Json<Upload>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(attachment::upload(&s.store, &a, &c, &k, &d, cmd).await)
}
fn file(r: attachment::Result<File>) -> Response {
    match r {
        Err(e) => Failure(e).into_response(),
        Ok(f) => {
            let encoded = f
                .metadata
                .filename
                .as_bytes()
                .iter()
                .map(|b| format!("%{b:02X}"))
                .collect::<String>();
            let mut r = f.content.into_response();
            r.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            r.headers_mut().insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            r.headers_mut().insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!(
                    "attachment; filename=download; filename*=UTF-8''{encoded}"
                ))
                .expect("encoded filename"),
            );
            r
        }
    }
}
async fn download(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, d, f)): Path<(String, String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    file(s.store.download(&a, &c, &k, &d, &f, None).await)
}
async fn historical(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, d, delivery, f)): Path<(String, String, String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, false).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    file(s.store.download(&a, &c, &k, &d, &f, Some(&delivery)).await)
}
async fn delete(
    State(s): State<S>,
    h: HeaderMap,
    Path((c, k, d, f)): Path<(String, String, String, String)>,
) -> Response {
    let a = match actor(&s, &h, true).await {
        Ok(a) => a,
        Err(e) => return e.into_response(),
    };
    response(
        s.store
            .delete(&a, &c, &k, &d, &f)
            .await
            .map(|_| serde_json::json!({"deleted":true})),
    )
}

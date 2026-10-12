use faktura_application::{
    auth, company,
    sales::{self, Create, Data, Error, Kind, Update},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany, sales::PgSales};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> Data {
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.tax_on = None;
    invoice.calculation.lines[0].unit_price = "1200".into();
    invoice.calculation.lines[0].quantity = "2".into();
    Data {
        name: "Test quotation".into(),
        maturity_days: 14,
        status: "created".into(),
        invoice,
    }
}
fn create(kind: Kind, key: &str) -> Create {
    Create {
        request_id: key.into(),
        kind,
        data: data(),
    }
}
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        pool.clone(),
        Some("synthetic-catalog-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-catalog-bootstrap-key-only-tests",
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

use faktura_application::delivery::{self, SendQuote};
use faktura_infrastructure::delivery::{MailConfig, PgDelivery};
use std::sync::Arc;
fn command(revision: i64, key: &str) -> SendQuote {
    SendQuote {
        attachment_ids: vec![],
        share_id: None,
        request_id: key.into(),
        revision,
        recipient: "customer@example.test".into(),
        subject: "Žluťoučká nabídka – zkouška".into(),
        body: "Dobrý den,\n\nv příloze nabídka.\nDěkujeme.".into(),
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

use faktura_application::share::{self, Create as CreateShare};
use faktura_infrastructure::share::PgShare;
fn share_cmd(revision: i64) -> CreateShare {
    CreateShare {
        request_id: KEY.into(),
        revision,
        expires_days: 30,
    }
}
fn bearer(link: &share::Link) -> &str {
    link.url.rsplit('/').next().unwrap()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn live_content_views_revocation_and_scope(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let sales = PgSales(pool.clone());
    let mut input = create(Kind::Quote, KEY);
    input.data.invoice.note = "INTERNAL SECRET".into();
    let q = sales::create(&sales, &a, &c, input).await.unwrap();
    let store = PgShare {
        pool: pool.clone(),
        base_url: Some("https://app.example.test".into()),
    };
    let (x, y) = tokio::join!(
        share::create(&store, &a, &c, &q.id, share_cmd(1)),
        share::create(&store, &a, &c, &q.id, share_cmd(1))
    );
    let link = x.unwrap();
    assert_eq!(link.id, y.unwrap().id);
    let token = bearer(&link);
    assert_eq!(token.len(), 64);
    assert!(matches!(
        share::list(&store, &a, &other, &q.id, 1).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        share::revoke(&store, &a, &other, &q.id, &link.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        share::public_quote(&store, "bad").await,
        Err(Error::NotFound)
    ));
    let public = share::public_quote(&store, token).await.unwrap();
    assert!(public.invoice.note.is_empty());
    assert!(public.invoice.contact_id.is_none());
    let json = serde_json::to_value(&public).unwrap();
    assert!(json.get("company_id").is_none());
    assert!(json.get("id").is_none());
    assert!(share::public_pdf(&store, token)
        .await
        .unwrap()
        .starts_with(b"%PDF"));
    assert_eq!(
        share::list(&store, &a, &c, &q.id, 1).await.unwrap().items[0].view_count,
        0
    );
    let (x, y) = tokio::join!(
        share::view(&store, token, KEY, 1),
        share::view(&store, token, KEY, 1)
    );
    x.unwrap();
    y.unwrap();
    assert_eq!(
        share::views(&store, &a, &c, &q.id, &link.id, 1)
            .await
            .unwrap()
            .total,
        1
    );
    let mut data = q.data;
    data.name = "Changed after sharing".into();
    let q = sales::update(
        &sales,
        &a,
        &c,
        &q.id,
        Update {
            revision: 1,
            archived: false,
            data,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        share::public_quote(&store, token).await.unwrap().name,
        "Changed after sharing"
    );
    assert!(matches!(
        share::view(&store, token, KEY, 100).await,
        Err(Error::Invalid(_))
    ));
    for n in 2..=65 {
        share::view(
            &store,
            token,
            &format!("00000000-0000-4000-8000-{n:012}"),
            2,
        )
        .await
        .unwrap();
    }
    let h = share::views(&store, &a, &c, &q.id, &link.id, 1)
        .await
        .unwrap();
    assert_eq!(h.total, 60);
    assert_eq!(h.items.len(), 25);
    assert_eq!(
        share::views(&store, &a, &c, &q.id, &link.id, 3)
            .await
            .unwrap()
            .items
            .len(),
        10
    );
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('viewer@example.test','no-login') RETURNING id").fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(c.parse::<i64>().unwrap())
        .bind(viewer)
        .execute(&pool)
        .await
        .unwrap();
    assert!(share::list(&store, &viewer.to_string(), &c, &q.id, 1)
        .await
        .is_ok());
    assert!(matches!(
        share::create(&store, &viewer.to_string(), &c, &q.id, share_cmd(2)).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        share::revoke(&store, &viewer.to_string(), &c, &q.id, &link.id).await,
        Err(Error::Forbidden)
    ));
    let archived = sales::update(
        &sales,
        &a,
        &c,
        &q.id,
        Update {
            revision: 2,
            archived: true,
            data: q.data,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        share::public_quote(&store, token).await,
        Err(Error::NotFound)
    ));
    sales::update(
        &sales,
        &a,
        &c,
        &q.id,
        Update {
            revision: 3,
            archived: false,
            data: archived.data,
        },
    )
    .await
    .unwrap();
    assert!(share::public_quote(&store, token).await.is_ok());
    sqlx::query("UPDATE quote_share SET expires_at=now()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        share::public_quote(&store, token).await,
        Err(Error::NotFound)
    ));
    sqlx::query("UPDATE quote_share SET expires_at=now()+interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();
    share::revoke(&store, &a, &c, &q.id, &link.id)
        .await
        .unwrap();
    share::revoke(&store, &a, &c, &q.id, &link.id)
        .await
        .unwrap();
    assert!(matches!(
        share::public_quote(&store, token).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        share::public_pdf(&store, token).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        share::view(&store, token, KEY, 1).await,
        Err(Error::NotFound)
    ));
    let new = share::create(
        &store,
        &a,
        &c,
        &q.id,
        CreateShare {
            request_id: "00000000-0000-4000-8000-000000000099".into(),
            revision: 4,
            expires_days: 1,
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE company SET deleted_at=now() WHERE id=$1")
        .bind(c.parse::<i64>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        share::public_quote(&store, bearer(&new)).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL and local SMTP"]
async fn link_email_and_revoked_queue(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let q = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Quote, KEY))
        .await
        .unwrap();
    let shares = PgShare {
        pool: pool.clone(),
        base_url: Some("https://app.example.test".into()),
    };
    let link = share::create(&shares, &a, &c, &q.id, share_cmd(1))
        .await
        .unwrap();
    let (port, capture) = smtp(false).await;
    let store = PgDelivery {
        pool: pool.clone(),
        mail: Some(Arc::new(MailConfig::local_capture(port))),
    };
    let mut cmd = command(1, KEY);
    cmd.share_id = Some(link.id.clone());
    cmd.body = "Open the quotation below.".into();
    let queued = delivery::send(&store, &a, &c, &q.id, cmd.clone())
        .await
        .unwrap();
    assert_eq!(queued.share_id, Some(link.id.clone()));
    assert!(queued.body.contains(&link.url));
    assert!(store.process_one().await.unwrap());
    let mime = capture.await.unwrap();
    assert!(!mime.contains("application/pdf"));
    assert!(mime.replace("=\r\n", "").contains(bearer(&link)));
    let replay = delivery::send(&store, &a, &c, &q.id, cmd).await.unwrap();
    assert_eq!(queued.id, replay.id);
    let mut cmd = command(2, "00000000-0000-4000-8000-000000000003");
    cmd.share_id = Some(link.id.clone());
    delivery::send(&store, &a, &c, &q.id, cmd.clone())
        .await
        .unwrap();
    share::revoke(&shares, &a, &c, &q.id, &link.id)
        .await
        .unwrap();
    assert!(!store.process_one().await.unwrap());
    let h = delivery::history(&store, &a, &c, &q.id, 1).await.unwrap();
    assert_eq!(h.items[0].status, "failed");
    cmd.request_id = "00000000-0000-4000-8000-000000000099".into();
    assert!(matches!(
        delivery::send(&store, &a, &c, &q.id, cmd.clone()).await,
        Err(Error::NotFound)
    ));
    let otherq = sales::create(&PgSales(pool.clone()), &a, &other, create(Kind::Quote, KEY))
        .await
        .unwrap();
    cmd.revision = 1;
    assert!(matches!(
        delivery::send(&store, &a, &other, &otherq.id, cmd).await,
        Err(Error::NotFound)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_public_access_and_csrf(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (a, session, c, _) = setup(&pool).await;
    let q = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Quote, KEY))
        .await
        .unwrap();
    let store = PgShare {
        pool: pool.clone(),
        base_url: Some("https://app.example.test".into()),
    };
    let app = faktura_server::share::router(
        store.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    let path = format!("/api/companies/{c}/sales/{}/shares", q.id);
    for (cookie, origin, expected) in [
        ("", "https://app.example.test", StatusCode::UNAUTHORIZED),
        (
            session.as_str(),
            "https://evil.example.test",
            StatusCode::UNAUTHORIZED,
        ),
        (session.as_str(), "https://app.example.test", StatusCode::OK),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&path)
                    .method("POST")
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&share_cmd(1)).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
    }
    let link = share::list(&store, &a, &c, &q.id, 1)
        .await
        .unwrap()
        .items
        .remove(0);
    for (endpoint, expected) in [
        ("resolve", StatusCode::OK),
        ("pdf", StatusCode::OK),
        ("view", StatusCode::NO_CONTENT),
    ] {
        let mut body = serde_json::json!({"token":bearer(&link)});
        if endpoint == "view" {
            body["visit_id"] = KEY.into();
            body["revision"] = 1.into();
        }
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/public/quotes/{endpoint}"))
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), expected);
        assert_eq!(r.headers()["cache-control"], "no-store");
        assert_eq!(r.headers()["referrer-policy"], "no-referrer");
        if endpoint == "resolve" {
            let v: serde_json::Value =
                serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
            assert_eq!(v["number"], q.number.clone().unwrap());
        }
    }
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/public/quotes/view")
                .method("HEAD")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::METHOD_NOT_ALLOWED);
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server =
        faktura_server::mcp::CompanyMcp::new(pool.clone(), session).with_share(store.clone());
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
    let response = client
        .call_tool(
            CallToolRequestParams::new("sales_sharelist").with_arguments(
                serde_json::json!({"company_id":c,"id":q.id})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(response.is_error, Some(false));
    let v: serde_json::Value =
        serde_json::from_str(&response.content[0].as_text().unwrap().text).unwrap();
    assert_eq!(v["items"][0]["view_count"], 1);
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_preserves_attachment_replay(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            all.iter()
                .filter(|m| m.version < 202610110014)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let q = sales::create(&PgSales(pool.clone()), &a, &c, create(Kind::Quote, KEY))
        .await
        .unwrap();
    let mut cmd = serde_json::to_value(command(1, KEY)).unwrap();
    cmd.as_object_mut().unwrap().remove("share_id");
    sqlx::query("INSERT INTO quote_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id) VALUES($1,$2,$3,$4::text::uuid,$5,1,$6,$7,'sender@example.test','','customer@example.test','Original','Original','test-message-id')").bind(c.parse::<i64>().unwrap()).bind(q.id.parse::<i64>().unwrap()).bind(a.parse::<i64>().unwrap()).bind(KEY).bind(cmd).bind(serde_json::to_value(&q).unwrap()).bind(b"original historical PDF".as_slice()).execute(&pool).await.unwrap();
    all.run(&pool).await.unwrap();
    let delivery = PgDelivery {
        pool: pool.clone(),
        mail: None,
    };
    let replay = delivery::send(&delivery, &a, &c, &q.id, command(1, KEY))
        .await
        .unwrap();
    assert!(replay.share_id.is_none());
    assert_eq!(
        delivery::pdf(&delivery, &a, &c, &q.id, Some(&replay.id))
            .await
            .unwrap(),
        b"original historical PDF"
    );
    assert_eq!(
        share::list(
            &PgShare {
                pool,
                base_url: None
            },
            &a,
            &c,
            &q.id,
            1
        )
        .await
        .unwrap()
        .total,
        0
    );
}

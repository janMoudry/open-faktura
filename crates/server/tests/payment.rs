#[path = "support/historical_documents.rs"]
mod historical_documents;
use faktura_application::{
    auth, company, draft,
    payment::{self, Error, PaymentInput, Register, Reverse},
};
use faktura_domain::{
    company::CompanySettings,
    invoice::{DraftData, TaxMode},
};
use faktura_infrastructure::{
    auth::PgAuth, company::PgCompany, draft::PgDraft, payment::PgPayment,
};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn key(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}
async fn setup(pool: &PgPool) -> (String, String, String, String) {
    let a = PgAuth::new(
        pool.clone(),
        Some("synthetic-payment-bootstrap-key-only-tests".into()),
    );
    auth::bootstrap(
        &a,
        "synthetic-payment-bootstrap-key-only-tests",
        "owner@payments.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (actor, token) = auth::login(&a, "owner@payments.test", "Synthetic password 123!")
        .await
        .unwrap();
    let year: i32 = sqlx::query_scalar("SELECT extract(year FROM now())::integer")
        .fetch_one(pool)
        .await
        .unwrap();
    let s = CompanySettings {
        number_year: year,
        ..Default::default()
    };
    let c = company::create(&PgCompany(pool.clone()), &actor.id, KEY, s.clone())
        .await
        .unwrap();
    let other = company::create(&PgCompany(pool.clone()), &actor.id, &key(2), s)
        .await
        .unwrap();
    (actor.id, token, c.id, other.id)
}
async fn issued(pool: &PgPool, actor: &str, company: &str, n: u32) -> draft::Draft {
    issued_case(pool, actor, company, n, 0, false).await
}
async fn issued_case(
    pool: &PgPool,
    actor: &str,
    company: &str,
    n: u32,
    days: i32,
    zero: bool,
) -> draft::Draft {
    let mut d: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    let today: String =
        sqlx::query_scalar("SELECT ((now() AT TIME ZONE 'Europe/Prague')::date-$1)::text")
            .bind(days)
            .fetch_one(pool)
            .await
            .unwrap();
    d.issued_on = Some(today.parse().unwrap());
    d.due_on = d.issued_on;
    d.tax_on = None;
    d.supplier.registration_number = "12345678".into();
    d.payment.method = "cash".into();
    d.calculation.tax_mode = TaxMode::NonPayer;
    d.calculation.lines[0].vat_percent = "0".into();
    if zero {
        d.calculation.lines[0].unit_price = "0".into();
    }
    let year = d.issued_on.unwrap().format("%Y").to_string();
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{number_year}',$2::text::jsonb) WHERE id=$1::text::bigint").bind(company).bind(year).execute(pool).await.unwrap();
    let store = PgDraft(pool.clone());
    let draft = draft::create(
        &store,
        actor,
        company,
        draft::Create {
            request_id: key(n),
            data: d,
        },
    )
    .await
    .unwrap();
    draft::issue(
        &store,
        actor,
        company,
        &draft.id,
        draft::Issue {
            request_id: key(n),
            revision: 1,
        },
    )
    .await
    .unwrap()
}
fn input(amount: &str, date: &str) -> PaymentInput {
    PaymentInput {
        amount: amount.into(),
        paid_on: date.parse().unwrap(),
        method: "bank_transfer".into(),
        note: "Synthetic payment".into(),
    }
}
fn register(n: u32, rev: i64, amount: &str, date: &str) -> Register {
    Register {
        request_id: key(n),
        revision: rev,
        payment: input(amount, date),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn partial_full_overpaid_correction_replay_and_pdf(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let inv = issued(&pool, &a, &c, 1).await;
    let s = PgPayment(pool.clone());
    let initial = payment::get(&s, &a, &c, &inv.id).await.unwrap();
    let day = initial.summary.today;
    assert_eq!(initial.summary.status, "unpaid");
    assert_eq!(initial.summary.outstanding, "59.99");
    assert!(!initial.summary.overdue);
    let pdf = draft::pdf(&PgDraft(pool.clone()), &a, &c, &inv.id)
        .await
        .unwrap();
    let p1 = payment::register(&s, &a, &c, &inv.id, register(1, 0, "20.00", &day))
        .await
        .unwrap();
    assert_eq!(p1.summary.status, "partial");
    assert_eq!(p1.summary.outstanding, "39.99");
    let replay = payment::register(&s, &a, &c, &inv.id, register(1, 0, "20.00", &day))
        .await
        .unwrap();
    assert_eq!(replay.items[0].id, p1.items[0].id);
    assert!(matches!(
        payment::register(&s, &a, &c, &inv.id, register(1, 0, "21.00", &day)).await,
        Err(Error::Conflict)
    ));
    let p2 = payment::register(&s, &a, &c, &inv.id, register(2, 1, "39.99", &day))
        .await
        .unwrap();
    assert_eq!(p2.summary.status, "paid");
    assert_eq!(p2.summary.outstanding, "0.00");
    let p3 = payment::register(&s, &a, &c, &inv.id, register(3, 2, "10.00", &day))
        .await
        .unwrap();
    assert_eq!(p3.summary.status, "overpaid");
    assert_eq!(p3.summary.overpayment, "10.00");
    let overview = payment::overview(&s, &a, &c).await.unwrap();
    assert_eq!(overview.received, "69.99");
    assert_eq!(overview.overpayment, "10.00");
    let p4 = payment::reverse(
        &s,
        &a,
        &c,
        &inv.id,
        &p3.items[0].id,
        Reverse {
            request_id: key(4),
            revision: 3,
            reason: "Duplicate entry".into(),
            replacement: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(p4.summary.status, "paid");
    assert!(p4.items[0].reversed_at.is_some());
    let p5 = payment::reverse(
        &s,
        &a,
        &c,
        &inv.id,
        &p1.items[0].id,
        Reverse {
            request_id: key(5),
            revision: 4,
            reason: "Wrong amount".into(),
            replacement: Some(input("10.00", &day)),
        },
    )
    .await
    .unwrap();
    assert_eq!(p5.summary.paid, "49.99");
    assert_eq!(p5.summary.outstanding, "10.00");
    assert_eq!(p5.summary.revision, 6);
    assert!(p5
        .items
        .iter()
        .find(|p| p.id == p1.items[0].id)
        .unwrap()
        .replacement_id
        .is_some());
    assert_eq!(
        draft::pdf(&PgDraft(pool.clone()), &a, &c, &inv.id)
            .await
            .unwrap(),
        pdf
    );
    let final_invoice = draft::get(&PgDraft(pool.clone()), &a, &c, &inv.id)
        .await
        .unwrap();
    assert_eq!(final_invoice.revision, inv.revision);
    assert_eq!(final_invoice.settlement.unwrap().paid, "49.99");
    assert!(sqlx::query("UPDATE invoice_payment SET amount=1")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("DELETE FROM invoice_payment_reversal")
        .execute(&pool)
        .await
        .is_err());
    assert_eq!(
        payment::overview(&s, &a, &c).await.unwrap().outstanding,
        "10.00"
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn concurrency_roles_isolation_dates_and_filters(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let inv = issued(&pool, &a, &c, 1).await;
    let s = PgPayment(pool.clone());
    let day = payment::get(&s, &a, &c, &inv.id)
        .await
        .unwrap()
        .summary
        .today;
    let (x, y) = tokio::join!(
        payment::register(&s, &a, &c, &inv.id, register(1, 0, "59.99", &day)),
        payment::register(&s, &a, &c, &inv.id, register(2, 0, "59.99", &day))
    );
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    let p = payment::get(&s, &a, &c, &inv.id).await.unwrap();
    assert_eq!(p.items.len(), 1);
    assert_eq!(p.summary.paid, "59.99");
    assert!(matches!(
        payment::get(&s, &a, &other, &inv.id).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        payment::register(&s, &a, &other, &inv.id, register(3, 0, "1", &day)).await,
        Err(Error::NotFound)
    ));
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES('viewer@payments.test','no-login',true) RETURNING id").fetch_one(&pool).await.unwrap();
    let viewer = viewer.to_string();
    assert!(matches!(
        payment::overview(&s, &viewer, &c).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1::text::bigint,$2::text::bigint,'viewer')").bind(&c).bind(&viewer).execute(&pool).await.unwrap();
    assert!(payment::get(&s, &viewer, &c, &inv.id).await.is_ok());
    assert!(matches!(
        payment::register(&s, &viewer, &c, &inv.id, register(4, 1, "1", &day)).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        payment::reverse(
            &s,
            &viewer,
            &c,
            &inv.id,
            &p.items[0].id,
            Reverse {
                request_id: key(5),
                revision: 1,
                reason: "test".into(),
                replacement: None
            }
        )
        .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        payment::register(&s, &a, &c, &inv.id, register(6, 1, "1", "2999-01-01")).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        payment::register(&s, &a, &c, &inv.id, register(7, 1, "1", "2000-01-01")).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        payment::register(&s, &a, &c, &inv.id, register(8, 1, "1.001", &day)).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        payment::overview(&s, &a, &other).await.unwrap().received,
        "0.00"
    );
    assert_eq!(
        draft::list(
            &PgDraft(pool.clone()),
            &a,
            &c,
            draft::Search {
                status: "paid".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        draft::list(
            &PgDraft(pool.clone()),
            &a,
            &c,
            draft::Search {
                status: "outstanding".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn overdue_zero_drafts_reversal_and_atomic_failure(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let s = PgPayment(pool.clone());
    let overdue = issued_case(&pool, &a, &c, 1, 10, false).await;
    let zero = issued_case(&pool, &a, &c, 2, 10, true).await;
    let day = payment::get(&s, &a, &c, &overdue.id)
        .await
        .unwrap()
        .summary
        .today;
    assert!(
        payment::get(&s, &a, &c, &overdue.id)
            .await
            .unwrap()
            .summary
            .overdue
    );
    let z = payment::get(&s, &a, &c, &zero.id).await.unwrap();
    assert_eq!(z.summary.status, "paid");
    assert!(!z.summary.overdue);
    assert!(matches!(
        payment::register(&s, &a, &c, &zero.id, register(2, 0, "1", &day)).await,
        Err(Error::Invalid(_))
    ));
    let partial = payment::register(&s, &a, &c, &overdue.id, register(1, 0, "20", &day))
        .await
        .unwrap();
    let sum = payment::overview(&s, &a, &c).await.unwrap();
    assert_eq!(sum.overdue, "39.99");
    assert_eq!(sum.overdue_count, 1);
    assert_eq!(sum.outstanding, "39.99");
    assert!(matches!(
        payment::reverse(
            &s,
            &a,
            &c,
            &overdue.id,
            &partial.items[0].id,
            Reverse {
                request_id: key(3),
                revision: 1,
                reason: "Wrong date".into(),
                replacement: Some(input("30", "2999-01-01"))
            }
        )
        .await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        payment::get(&s, &a, &c, &overdue.id)
            .await
            .unwrap()
            .summary
            .paid,
        "20.00"
    );
    let cmd = || Reverse {
        request_id: key(4),
        revision: 1,
        reason: "Mistake".into(),
        replacement: None,
    };
    let (x, y) = tokio::join!(
        payment::reverse(&s, &a, &c, &overdue.id, &partial.items[0].id, cmd()),
        payment::reverse(&s, &a, &c, &overdue.id, &partial.items[0].id, cmd())
    );
    assert_eq!(x.unwrap().summary.revision, y.unwrap().summary.revision);
    assert_eq!(
        payment::overview(&s, &a, &c).await.unwrap().overdue,
        "59.99"
    );
    let mut d: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    d.customer = None;
    let draft = draft::create(
        &PgDraft(pool.clone()),
        &a,
        &c,
        draft::Create {
            request_id: key(8),
            data: d,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        payment::register(&s, &a, &c, &draft.id, register(8, 0, "1", &day)).await,
        Err(Error::Conflict)
    ));
    assert_eq!(payment::overview(&s, &a, &c).await.unwrap().draft_count, 1);
    assert_eq!(
        draft::list(
            &PgDraft(pool),
            &a,
            &c,
            draft::Search {
                status: "overdue".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn rest_mcp_payment_equivalence_and_csrf(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as McpServiceExt};
    use tower::ServiceExt;
    let (a, token, c, _) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c, 1).await;
    let day = payment::get(&PgPayment(pool.clone()), &a, &c, &invoice.id)
        .await
        .unwrap()
        .summary
        .today;
    let cmd = register(1, 0, "12.34", &day);
    let body = serde_json::to_value(&cmd).unwrap();
    let app = faktura_server::invoice::router(pool.clone(), vec!["https://app.test".into()], false);
    let mut saved = None;
    for (origin, cookie, status) in [
        ("https://evil.test", token.as_str(), 401),
        ("https://app.test", "", 401),
        ("https://app.test", token.as_str(), 200),
    ] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/companies/{c}/invoices/{}/payments",
                        invoice.id
                    ))
                    .header("origin", origin)
                    .header("cookie", format!("faktura_session={cookie}"))
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), status);
        if status == 200 {
            saved = Some(
                serde_json::from_slice::<serde_json::Value>(
                    &to_bytes(r.into_body(), 100000).await.unwrap(),
                )
                .unwrap(),
            );
        }
    }
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
    let mut args = body.as_object().unwrap().clone();
    args.insert("company_id".into(), c.clone().into());
    args.insert("id".into(), invoice.id.clone().into());
    let result = client
        .call_tool(CallToolRequestParams::new("invoice_register_payment").with_arguments(args))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let encoded = serde_json::to_value(result).unwrap();
    let result: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(result, saved.unwrap());
    let payment_id = result["items"][0]["id"].as_str().unwrap();
    let args = serde_json::json!({"company_id":c,"id":invoice.id,"payment_id":payment_id,"request_id":key(2),"revision":1,"reason":"Synthetic reversal","replacement":null});
    assert_eq!(
        client
            .call_tool(
                CallToolRequestParams::new("invoice_reverse_payment")
                    .with_arguments(args.as_object().unwrap().clone())
            )
            .await
            .unwrap()
            .is_error,
        Some(false)
    );
    let args = serde_json::json!({"company_id":c});
    let result = client
        .call_tool(
            CallToolRequestParams::new("company_overview")
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));
    let encoded = serde_json::to_value(result).unwrap();
    let result: serde_json::Value =
        serde_json::from_str(encoded["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(result["received"], "0.00");
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[sqlx::test(migrations = false)]
#[ignore = "requires isolated PostgreSQL"]
async fn upgrade_p07_preserves_invoice_and_pdf(pool: PgPool) {
    let all = sqlx::migrate!("../../migrations");
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(all.iter().take(8).cloned().collect()),
        ..sqlx::migrate!("../../migrations")
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    let mut data: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.supplier.registration_number = "12345678".into();
    data.payment.method = "cash".into();
    data.calculation.tax_mode = TaxMode::NonPayer;
    data.calculation.lines[0].vat_percent = "0".into();
    data.tax_on = None;
    let (invoice, pdf) = historical_documents::invoice(
        &pool,
        &a,
        &c,
        draft::Create {
            request_id: key(1),
            data,
        },
    )
    .await;
    all.run(&pool).await.unwrap();
    let state = payment::get(&PgPayment(pool.clone()), &a, &c, &invoice.id)
        .await
        .unwrap();
    assert_eq!(state.summary.paid, "0.00");
    assert_eq!(state.summary.outstanding, "59.99");
    assert!(state.items.is_empty());
    assert_eq!(
        draft::pdf(&PgDraft(pool), &a, &c, &invoice.id)
            .await
            .unwrap(),
        pdf
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL and local development Compose"]
async fn restore_database_pdf_and_attachment_into_clean_instance(pool: PgPool) {
    use faktura_application::auth::AuthStore;
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let (a, token, c, _) = setup(&pool).await;
    let invoice = issued(&pool, &a, &c, 1).await;
    let store = PgPayment(pool.clone());
    let day = payment::get(&store, &a, &c, &invoice.id)
        .await
        .unwrap()
        .summary
        .today;
    let p = payment::register(&store, &a, &c, &invoice.id, register(1, 0, "20.00", &day))
        .await
        .unwrap();
    payment::reverse(
        &store,
        &a,
        &c,
        &invoice.id,
        &p.items[0].id,
        Reverse {
            request_id: key(2),
            revision: 1,
            reason: "Corrected during recovery drill".into(),
            replacement: Some(input("25.00", &day)),
        },
    )
    .await
    .unwrap();
    let expected =
        serde_json::to_value(payment::get(&store, &a, &c, &invoice.id).await.unwrap()).unwrap();
    let pdf = draft::pdf(&PgDraft(pool.clone()), &a, &c, &invoice.id)
        .await
        .unwrap();
    let source: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let target = format!("p08_restore_{suffix}");
    fn docker() -> Command {
        let mut c = Command::new("docker");
        c.current_dir("../..").args([
            "compose",
            "-f",
            "infra/compose.yaml",
            "exec",
            "-T",
            "postgres",
        ]);
        c
    }
    let dump = docker()
        .args(["pg_dump", "-U", "faktura", "-d", &source, "--format=custom"])
        .output()
        .unwrap();
    assert!(dump.status.success());
    let admin = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {target}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut restore = docker()
        .args([
            "pg_restore",
            "-U",
            "faktura",
            "-d",
            &target,
            "--exit-on-error",
            "--single-transaction",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    restore
        .stdin
        .take()
        .unwrap()
        .write_all(&dump.stdout)
        .unwrap();
    assert!(restore.wait().unwrap().success());
    let options: sqlx::postgres::PgConnectOptions =
        std::env::var("DATABASE_URL").unwrap().parse().unwrap();
    let restored = PgPool::connect_with(options.database(&target))
        .await
        .unwrap();
    sqlx::query("DELETE FROM auth_session")
        .execute(&restored)
        .await
        .unwrap();
    assert!(PgAuth::new(restored.clone(), None)
        .session(&token)
        .await
        .is_err());
    assert_eq!(
        serde_json::to_value(
            payment::get(&PgPayment(restored.clone()), &a, &c, &invoice.id)
                .await
                .unwrap()
        )
        .unwrap(),
        expected
    );
    assert_eq!(
        draft::pdf(&PgDraft(restored.clone()), &a, &c, &invoice.id)
            .await
            .unwrap(),
        pdf
    );
    assert_eq!(
        payment::overview(&PgPayment(restored.clone()), &a, &c)
            .await
            .unwrap()
            .outstanding,
        "34.99"
    );
    // External attachments are not yet product data; exercise the backup file archive with a synthetic file.
    let root = std::env::temp_dir().join(&target);
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::create_dir(root.join("restored")).unwrap();
    std::fs::write(
        root.join("source/proof.txt"),
        b"Synthetic attachment for P08 restore drill",
    )
    .unwrap();
    assert!(Command::new("tar")
        .args(["-cf"])
        .arg(root.join("files.tar"))
        .arg("-C")
        .arg(root.join("source"))
        .arg(".")
        .status()
        .unwrap()
        .success());
    assert!(Command::new("tar")
        .args(["-xf"])
        .arg(root.join("files.tar"))
        .arg("-C")
        .arg(root.join("restored"))
        .status()
        .unwrap()
        .success());
    assert_eq!(
        std::fs::read(root.join("restored/proof.txt")).unwrap(),
        std::fs::read(root.join("source/proof.txt")).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
    restored.close().await;
    sqlx::query(&format!("DROP DATABASE {target} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

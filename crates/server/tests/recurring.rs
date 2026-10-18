use faktura_application::{auth, company, sales::Data};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{auth::PgAuth, company::PgCompany};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
fn data() -> Data {
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.supplier.vat_number = "CZ12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::VatPayer;
    invoice.calculation.lines[0].vat_percent = "21".into();
    invoice.tax_on = None;
    invoice.calculation.lines[0].unit_price = "1000".into();
    invoice.calculation.lines[0].quantity = "1".into();
    Data {
        name: "Test quotation".into(),
        maturity_days: 14,
        status: "created".into(),
        invoice,
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
        CompanySettings {
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..CompanySettings::default()
        },
    )
    .await
    .unwrap();
    let second = company::create(
        &PgCompany(pool.clone()),
        &actor.id,
        "00000000-0000-4000-8000-000000000002",
        CompanySettings {
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..CompanySettings::default()
        },
    )
    .await
    .unwrap();
    (actor.id, token, first.id, second.id)
}

use faktura_application::recurring::{self, Action, Command, Create, RecurringStore};
use faktura_infrastructure::recurring::PgRecurring;
fn cmd(revision: i64, action: Action) -> Command {
    Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        revision,
        action,
    }
}
async fn rule_data(pool: &PgPool) -> recurring::Data {
    let today: sqlx::types::chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(pool)
            .await
            .unwrap();
    recurring::Data {
        rate_policy: recurring::RatePolicy::Stored,
        name: "Monthly service".into(),
        schedule: faktura_domain::recurring::Schedule {
            first_on: today,
            frequency: "month".into(),
            interval: 1,
            month_end: false,
            end_on: None,
            count: Some(1),
            timezone: "Europe/Prague".into(),
        },
        maturity_days: 14,
        auto_issue: true,
        mail: None,
        invoice: data().invoice,
    }
}
async fn create(s: &PgRecurring, a: &str, c: &str, d: recurring::Data) -> recurring::Rule {
    recurring::create(
        s,
        a,
        c,
        Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            data: d,
        },
    )
    .await
    .unwrap()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_occurrence_replay_pause_and_snapshots(pool: PgPool) {
    let (a, _, c, other) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let d = rule_data(&pool).await;
    let mut v = create(&s, &a, &c, d.clone()).await;
    assert_eq!(v.status, "paused");
    assert!(!s.process_one().await.unwrap());
    assert!(s.get(&a, &other, &v.id).await.is_err());
    v = recurring::command(&s, &a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    let run = cmd(v.revision, Action::Run);
    let request = serde_json::to_value(&run).unwrap();
    let (first, second) = tokio::join!(
        recurring::command(&s, &a, &c, &v.id, run),
        recurring::command(
            &s,
            &a,
            &c,
            &v.id,
            serde_json::from_value(request.clone()).unwrap()
        )
    );
    let v = first.unwrap();
    assert_eq!(
        second.unwrap().occurrences[0].invoice_id,
        v.occurrences[0].invoice_id
    );
    assert_eq!(v.generated, 1);
    assert_eq!(v.status, "completed");
    assert!(v.occurrences[0].number.is_some());
    assert!(!s.process_one().await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let mut changed = d;
    changed.name = "Future change".into();
    changed.schedule.count = Some(2);
    let saved = recurring::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            v.revision,
            Action::Save {
                data: Box::new(changed),
            },
        ),
    )
    .await
    .unwrap();
    let snapshot: String = sqlx::query_scalar("SELECT snapshot->>'name' FROM recurring_occurrence")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(snapshot, "Monthly service");
    assert!(s
        .command(&a, &c, &v.id, serde_json::from_value(request).unwrap())
        .await
        .is_ok());
    let mut invalid = saved.data.clone();
    invalid.schedule.interval = 2;
    assert!(recurring::command(
        &s,
        &a,
        &c,
        &v.id,
        cmd(
            saved.revision,
            Action::Save {
                data: Box::new(invalid)
            }
        )
    )
    .await
    .is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn missed_dates_drafts_and_revoked_permissions(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let mut d = rule_data(&pool).await;
    d.auto_issue = false;
    d.schedule.frequency = "day".into();
    d.schedule.first_on = d.schedule.first_on.pred_opt().unwrap();
    d.schedule.count = Some(2);
    let mut v = create(&s, &a, &c, d).await;
    v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    v = s.get(&a, &c, &v.id).await.unwrap();
    assert_eq!(v.generated, 1);
    assert!(v.occurrences[0].number.is_none());
    v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Pause))
        .await
        .unwrap();
    assert!(!s.process_one().await.unwrap());
    v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&a).execute(&pool).await.unwrap();
    assert!(s.process_one().await.unwrap());
    v = s.get(&a, &c, &v.id).await.unwrap();
    assert_eq!(v.status, "error");
    assert_eq!(v.generated, 1);
    assert_eq!(v.error_code.as_deref(), Some("permission"));
    assert!(s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .is_err());
    sqlx::query("UPDATE membership SET role='owner' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&a).execute(&pool).await.unwrap();
    let v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    let v = s.get(&a, &c, &v.id).await.unwrap();
    assert_eq!(v.generated, 2);
    assert_eq!(v.status, "completed");
    assert_ne!(v.occurrences[0].scheduled_on, v.occurrences[1].scheduled_on);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn failed_issue_rolls_back_draft_and_cursor(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let mut d = rule_data(&pool).await;
    d.invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    d.invoice.calculation.lines[0].vat_percent = "0".into();
    let v = create(&s, &a, &c, d).await;
    let v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    let v = s.get(&a, &c, &v.id).await.unwrap();
    assert_eq!(v.generated, 0);
    assert_eq!(v.status, "error");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let mut fixed = v.data.clone();
    fixed.invoice = data().invoice;
    let v = s
        .command(
            &a,
            &c,
            &v.id,
            cmd(
                v.revision,
                Action::Save {
                    data: Box::new(fixed),
                },
            ),
        )
        .await
        .unwrap();
    let v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    assert_eq!(s.get(&a, &c, &v.id).await.unwrap().generated, 1);
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
#[ignore = "isolated PostgreSQL and loopback SMTP"]
async fn mail_is_separate_idempotent_and_survives_restart(pool: PgPool) {
    use faktura_infrastructure::{delivery::MailConfig, invoice_delivery::PgInvoiceDelivery};
    let (a, _, c, _) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let mut d = rule_data(&pool).await;
    d.mail = Some(recurring::Mail {
        recipient: "synthetic@example.test".into(),
        subject: "Synthetic recurring".into(),
        body: "Test only".into(),
    });
    let v = create(&s, &a, &c, d).await;
    let v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    s.process_one().await.unwrap();
    let no_mail = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: None,
    };
    s.enqueue_one(&no_mail).await.unwrap();
    assert_eq!(s.get(&a, &c, &v.id).await.unwrap().generated, 1);
    sqlx::query("UPDATE recurring_occurrence SET next_attempt=now()")
        .execute(&pool)
        .await
        .unwrap();
    let (port, capture) = smtp(false).await;
    let delivery = PgInvoiceDelivery {
        pool: pool.clone(),
        mail: Some(std::sync::Arc::new(MailConfig::local_capture(port))),
    };
    let resumed = PgRecurring(pool.clone());
    assert!(resumed.enqueue_one(&delivery).await.unwrap());
    // Simulate a crash after enqueue committed but before the occurrence link committed.
    sqlx::query("UPDATE recurring_occurrence SET delivery_id=NULL,next_attempt=now()")
        .execute(&pool)
        .await
        .unwrap();
    assert!(resumed.enqueue_one(&delivery).await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_delivery")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert!(delivery.process_one().await.unwrap());
    let mime = capture.await.unwrap();
    assert!(mime.contains("application/pdf"));
    assert_eq!(
        s.get(&a, &c, &v.id).await.unwrap().occurrences[0]
            .delivery_status
            .as_deref(),
        Some("sent")
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn rest_mcp_preview_and_command_parity(pool: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use rmcp::{model::CallToolRequestParams, ServiceExt as _};
    use tower::ServiceExt;
    let (a, token, c, other) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let v = create(&s, &a, &c, rule_data(&pool).await).await;
    let app = faktura_server::recurring::router(
        pool.clone(),
        vec!["http://localhost:5173".into()],
        false,
    );
    let action = cmd(v.revision, Action::Activate);
    let body = serde_json::to_vec(&action).unwrap();
    for (origin, status) in [("http://evil.test", 401), ("http://localhost:5173", 200)] {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/companies/{c}/recurring/{}", v.id))
                    .header("cookie", format!("faktura_session={token}"))
                    .header("origin", origin)
                    .header("x-faktura-request", "1")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), status);
        if status == 200 {
            let result: serde_json::Value =
                serde_json::from_slice(&to_bytes(r.into_body(), 1000000).await.unwrap()).unwrap();
            assert_eq!(result["status"], "active");
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
    let mut args = serde_json::to_value(action)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    args.insert("company_id".into(), c.clone().into());
    args.insert("id".into(), v.id.clone().into());
    let result = client
        .call_tool(CallToolRequestParams::new("recurring_command").with_arguments(args))
        .await
        .unwrap();
    assert_ne!(result.is_error, Some(true));
    let preview = client
        .call_tool(
            CallToolRequestParams::new("recurring_preview").with_arguments(
                serde_json::json!({"company_id":c,"schedule":v.data.schedule,"index":0})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_ne!(preview.is_error, Some(true));
    assert!(s.get(&a, &other, &v.id).await.is_err());
    client.cancel().await.unwrap();
    task.await.unwrap();
}
#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL"]
async fn upgrade_021_preserves_invoice_history(pool: PgPool) {
    let previous = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610120022)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    previous.run(&pool).await.unwrap();
    let (a, _, c, _) = setup(&pool).await;
    use faktura_application::draft::{self, DraftStore};
    let store = faktura_infrastructure::draft::PgDraft(pool.clone());
    let mut original = data().invoice;
    original.tax_on = original.issued_on;
    let d = draft::create(
        &store,
        &a,
        &c,
        draft::Create {
            request_id: KEY.into(),
            data: original,
        },
    )
    .await
    .unwrap();
    // Seed the historical document and its PDF before upgrading; today's
    // issuer intentionally requires the complete current schema.
    let mut d = d;
    d.number = Some(format!("LEGACY-{}", d.id));
    d.issued_at = Some(sqlx::types::chrono::Utc::now().to_rfc3339());
    d.revision += 1;
    let pdf = faktura_infrastructure::invoice_pdf::render(&d).unwrap();
    sqlx::query("UPDATE invoice_draft SET number=$3,issued_at=now(),issue_request_id=$4::text::uuid,issue_actor_id=$5::text::bigint,issue_revision=revision,revision=revision+1,pdf=$6 WHERE company_id=$1::text::bigint AND id=$2::text::bigint")
        .bind(&c).bind(&d.id).bind(&d.number).bind(KEY).bind(&a).bind(&pdf).execute(&pool).await.unwrap();
    faktura_infrastructure::migrate(&pool).await.unwrap();
    assert_eq!(store.pdf(&a, &c, &d.id).await.unwrap(), pdf);
    let s = PgRecurring(pool.clone());
    let v = create(&s, &a, &c, rule_data(&pool).await).await;
    assert_eq!(v.generated, 0);
    assert_eq!(store.get(&a, &c, &d.id).await.unwrap().number, d.number);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn worker_uses_saved_calendar_timezone(pool: PgPool) {
    let (a, _, c, _) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let mut d = rule_data(&pool).await;
    d.auto_issue = false;
    let east: sqlx::types::chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Pacific/Kiritimati')::date")
            .fetch_one(&pool)
            .await
            .unwrap();
    d.schedule.first_on = east;
    d.schedule.timezone = "Etc/GMT+12".into();
    let west = create(&s, &a, &c, d.clone()).await;
    let west = s
        .command(&a, &c, &west.id, cmd(west.revision, Action::Activate))
        .await
        .unwrap();
    assert!(!s.process_one().await.unwrap());
    d.schedule.timezone = "Pacific/Kiritimati".into();
    let east = create(&s, &a, &c, d).await;
    let east = s
        .command(&a, &c, &east.id, cmd(east.revision, Action::Activate))
        .await
        .unwrap();
    assert!(s.process_one().await.unwrap());
    assert_eq!(s.get(&a, &c, &east.id).await.unwrap().generated, 1);
    assert_eq!(s.get(&a, &c, &west.id).await.unwrap().generated, 0);
}

struct RateFixture {
    pool: PgPool,
    change_rule: Option<String>,
    fail: bool,
    calls: std::sync::atomic::AtomicUsize,
}
#[async_trait::async_trait]
impl faktura_application::exchange_rate::RateProvider for RateFixture {
    async fn lookup(
        &self,
        currency: &str,
        date: sqlx::types::chrono::NaiveDate,
    ) -> company::Result<faktura_application::exchange_rate::Rate> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(id) = &self.change_rule {
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                sqlx::query(
                    "UPDATE recurring_rule SET revision=revision+1 WHERE id=$1::text::bigint",
                )
                .bind(id)
                .execute(&self.pool),
            )
            .await
            .expect("provider I/O must hold no SQL row lock")
            .unwrap();
        }
        if self.fail {
            return Err(company::CompanyError::Unavailable);
        }
        Ok(faktura_application::exchange_rate::Rate {
            currency: currency.into(),
            requested_on: date,
            published_on: date,
            source: "CNB".into(),
            exchange_rate: faktura_domain::exchange::ExchangeRate {
                rate: "26.5".into(),
                units: "1".into(),
            },
        })
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn opt_in_cnb_rate_snapshot_failure_replay_and_revision_race(pool: PgPool) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let (a, _, c, _) = setup(&pool).await;
    let s = PgRecurring(pool.clone());
    let mut d = rule_data(&pool).await;
    d.rate_policy = recurring::RatePolicy::CnbIssueDate;
    d.invoice.payment.method = "bank_transfer".into();
    d.invoice.payment.bank_account = "19-2000145399/0800".into();
    d.invoice.calculation.currency = "EUR".into();
    d.invoice.calculation.exchange_rate = Some(faktura_domain::exchange::ExchangeRate {
        rate: "25".into(),
        units: "1".into(),
    });
    let v = create(&s, &a, &c, d.clone()).await;
    let v = s
        .command(&a, &c, &v.id, cmd(v.revision, Action::Activate))
        .await
        .unwrap();
    let unavailable = RateFixture {
        pool: pool.clone(),
        change_rule: None,
        fail: true,
        calls: AtomicUsize::new(0),
    };
    assert!(s
        .command_with_rate_provider(&a, &c, &v.id, cmd(v.revision, Action::Run), &unavailable)
        .await
        .is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let provider = RateFixture {
        pool: pool.clone(),
        change_rule: None,
        fail: false,
        calls: AtomicUsize::new(0),
    };
    let command = cmd(v.revision, Action::Run);
    let replay = serde_json::to_value(&command).unwrap();
    let result = s
        .command_with_rate_provider(&a, &c, &v.id, command, &provider)
        .await
        .unwrap();
    assert_eq!(result.generated, 1);
    assert_eq!(
        result.data.invoice.calculation.exchange_rate.unwrap().rate,
        "25"
    );
    let rate: String =
        sqlx::query_scalar("SELECT data#>>'{calculation,exchange_rate,rate}' FROM invoice_draft")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rate, "26.5");
    let again = s
        .command_with_rate_provider(
            &a,
            &c,
            &v.id,
            serde_json::from_value(replay).unwrap(),
            &unavailable,
        )
        .await
        .unwrap();
    assert_eq!(again.generated, 1);
    assert_eq!(unavailable.calls.load(Ordering::SeqCst), 1);
    let second = create(&s, &a, &c, d).await;
    let second = s
        .command(&a, &c, &second.id, cmd(second.revision, Action::Activate))
        .await
        .unwrap();
    let racing = RateFixture {
        pool: pool.clone(),
        change_rule: Some(second.id.clone()),
        fail: false,
        calls: AtomicUsize::new(0),
    };
    assert!(matches!(
        s.command_with_rate_provider(
            &a,
            &c,
            &second.id,
            cmd(second.revision, Action::Run),
            &racing
        )
        .await,
        Err(company::CompanyError::Conflict)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM invoice_draft")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

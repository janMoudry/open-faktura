use base64::Engine;
use faktura_application::{
    auth,
    bank_notification::{self, NotificationStore},
    banking::{self, BankStore},
    company,
};
use faktura_domain::{banking::AccountData, company::CompanySettings};
use faktura_infrastructure::{
    auth::PgAuth, bank_notification::PgNotification, banking::PgBank, company::PgCompany,
};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn opt_in_gateway_atomic_replay_scope_format_roles_and_no_matching(p: PgPool) {
    let auth = PgAuth::new(p.clone(), Some("synthetic-bank-gateway".into()));
    auth::bootstrap(
        &auth,
        "synthetic-bank-gateway",
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
    let bank = PgBank(p.clone());
    let banking::Reply::Account(account) = bank
        .execute(
            &a.id,
            &c.id,
            banking::Command {
                request_id: key(),
                action: banking::Action::Account {
                    id: None,
                    revision: None,
                    archived: false,
                    data: AccountData {
                        name: "Synthetic gateway account".into(),
                        iban: "CZ6508000000192000145399".into(),
                        currency: "CZK".into(),
                    },
                },
            },
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let s = PgNotification(p.clone());
    let intake = || bank_notification::Intake {
        request_id: key(),
        message_id: "synthetic-bank@example.test".into(),
        sender: "unverified@example.test".into(),
        file: banking::StatementInput {
            account_id: account.id.clone(),
            format: "camt053".into(),
            filename: "synthetic.xml".into(),
            content_base64: base64::engine::general_purpose::STANDARD
                .encode(include_bytes!("../../../tests/fixtures/bank/camt053.xml")),
        },
    };
    assert!(!s.settings(&a.id, &c.id, &account.id).await.unwrap().enabled);
    assert!(s.receive(&a.id, &c.id, intake()).await.is_err());
    assert!(s.settings(&a.id, &other.id, &account.id).await.is_err());
    s.configure(
        &a.id,
        &c.id,
        bank_notification::Settings {
            account_id: account.id.clone(),
            revision: 0,
            enabled: true,
            format: "camt053".into(),
        },
    )
    .await
    .unwrap();
    let audit:i64=sqlx::query_scalar("SELECT count(*) FROM bank_notification_settings_audit WHERE company_id=$1::text::bigint AND revision=1 AND enabled").bind(&c.id).fetch_one(&p).await.unwrap();
    assert_eq!(audit, 1);
    let report = s.receive(&a.id, &c.id, intake()).await.unwrap();
    assert_eq!(report.new_count, 2);
    assert_eq!(
        s.receive(&a.id, &c.id, intake()).await.unwrap().new_count,
        2
    );
    let rows = bank
        .movements(&a.id, &c.id, Default::default())
        .await
        .unwrap();
    assert_eq!(rows.total, 2);
    assert!(rows.items[0].matches.is_empty());
    let mut changed = intake();
    changed.sender = "changed@example.test".into();
    assert!(s.receive(&a.id, &c.id, changed).await.is_err());
    let mut invalid = intake();
    invalid.message_id = "wrong-bank".into();
    invalid.file.content_base64 = base64::engine::general_purpose::STANDARD.encode(
        include_str!("../../../tests/fixtures/bank/camt053.xml")
            .replace("CZ6508000000192000145399", "CZ5855000000001265098001"),
    );
    assert!(s.receive(&a.id, &c.id, invalid).await.is_err());
    assert_eq!(s.history(&a.id, &c.id, &account.id).await.unwrap().len(), 1);
    assert_eq!(
        bank.movements(&a.id, &c.id, Default::default())
            .await
            .unwrap()
            .total,
        2
    );
    let mut second = intake();
    second.message_id = "second-bank@example.test".into();
    let second_key = second.request_id.clone();
    let duplicate = s.receive(&a.id, &c.id, second).await.unwrap();
    assert_eq!(duplicate.duplicate_count, 2);
    assert_eq!(duplicate.new_count, 0);
    let mut collision = intake();
    collision.request_id = second_key;
    assert!(s.receive(&a.id, &c.id, collision).await.is_err());
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c.id)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.settings(&a.id, &c.id, &account.id).await.unwrap().enabled);
    assert!(s.receive(&a.id, &c.id, intake()).await.is_err());
    assert!(s
        .configure(
            &a.id,
            &c.id,
            bank_notification::Settings {
                account_id: account.id.clone(),
                revision: 1,
                enabled: false,
                format: "camt053".into()
            }
        )
        .await
        .is_err());
}

use faktura_application::{
    company::{self, CompanyError, CompanyStore},
    company_admin::*,
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{company::PgCompany, company_admin::PgAdministration};
use sqlx::PgPool;
fn command(revision: i64, action: Action) -> Command {
    Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        revision,
        action,
    }
}
async fn user(p: &PgPool, email: &str) -> String {
    sqlx::query_scalar::<_,i64>("INSERT INTO app_user(email,password_hash,installation_admin) VALUES($1,'synthetic-no-login',false) RETURNING id").bind(email).fetch_one(p).await.unwrap().to_string()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn transfer_leave_restore_replay_and_isolation(p: PgPool) {
    let owner = user(&p, "owner-admin@example.test").await;
    let member = user(&p, "member-admin@example.test").await;
    let outsider = user(&p, "outsider-admin@example.test").await;
    let companies = PgCompany(p.clone());
    let admin = PgAdministration(p.clone());
    let settings:CompanySettings=serde_json::from_value(serde_json::json!({"name":"Synthetic","entity_type":"individual","registration_number":"","vat_number":"","street":"","city":"","postal_code":"","country":"CZ","email":"","phone":"","tax_mode":"non_payer","tax_effective_from":"","currency":"CZK","timezone":"Europe/Prague","maturity_days":14,"payment_method":"bank_transfer","iban":"","number_prefix":"FV","number_year":2026,"next_number":1})).unwrap();
    let c = company::create(
        &companies,
        &owner,
        &uuid::Uuid::new_v4().to_string(),
        settings,
    )
    .await
    .unwrap();
    assert!(matches!(
        admin.list(&outsider, &c.id).await,
        Err(CompanyError::NotFound)
    ));
    assert!(matches!(
        admin
            .execute(&owner, &c.id, command(1, Action::Leave))
            .await,
        Err(CompanyError::Conflict)
    ));
    let add = command(
        1,
        Action::AddMember {
            email: "member-admin@example.test".into(),
            role: Role::Viewer,
        },
    );
    assert_eq!(
        admin
            .execute(&owner, &c.id, add.clone())
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(admin.execute(&owner, &c.id, add).await.unwrap().revision, 2);
    assert!(matches!(
        admin
            .execute(&member, &c.id, command(2, Action::Restore))
            .await,
        Err(CompanyError::Forbidden)
    ));
    let transfer = command(
        2,
        Action::Transfer {
            user_id: member.clone(),
        },
    );
    let next = admin
        .execute(&owner, &c.id, transfer.clone())
        .await
        .unwrap();
    assert_eq!(next.revision, 3);
    assert_eq!(
        next.members
            .iter()
            .find(|m| m.user_id == member)
            .unwrap()
            .role,
        "owner"
    );
    assert_eq!(
        admin
            .execute(&owner, &c.id, transfer)
            .await
            .unwrap()
            .revision,
        3
    );
    assert!(matches!(
        companies.delete(&owner, &c.id, 3).await,
        Err(CompanyError::Forbidden)
    ));
    let leave = command(3, Action::Leave);
    assert_eq!(
        admin
            .execute(&owner, &c.id, leave.clone())
            .await
            .unwrap()
            .revision,
        4
    );
    assert!(matches!(
        admin.execute(&owner, &c.id, leave).await,
        Err(CompanyError::NotFound)
    ));
    assert!(matches!(
        admin.list(&owner, &c.id).await,
        Err(CompanyError::NotFound)
    ));
    companies.delete(&member, &c.id, 4).await.unwrap();
    assert_eq!(admin.trash(&member).await.unwrap().len(), 1);
    assert!(admin.trash(&owner).await.unwrap().is_empty());
    let restored = admin
        .execute(&member, &c.id, command(5, Action::Restore))
        .await
        .unwrap();
    assert!(!restored.deleted);
    assert_eq!(restored.revision, 6);
    assert_eq!(companies.get(&member, &c.id).await.unwrap().revision, 6);
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM company_audit WHERE company_id=$1")
        .bind(c.id.parse::<i64>().unwrap())
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(audits, 6);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_transfer_has_one_owner(p: PgPool) {
    let a = user(&p, "a@example.test").await;
    let b = user(&p, "b@example.test").await;
    let c = user(&p, "c@example.test").await;
    // Construct a minimal company directly; this test targets concurrent administration.
    let company:i64=sqlx::query_scalar("INSERT INTO company(creator_id,request_id,initial_settings,settings) VALUES($1,$2::text::uuid,'{}','{}') RETURNING id").bind(a.parse::<i64>().unwrap()).bind(uuid::Uuid::new_v4().to_string()).fetch_one(&p).await.unwrap();
    for (u, role) in [(&a, "owner"), (&b, "editor"), (&c, "editor")] {
        sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,$3)")
            .bind(company)
            .bind(u.parse::<i64>().unwrap())
            .bind(role)
            .execute(&p)
            .await
            .unwrap();
    }
    let store = PgAdministration(p.clone());
    let cid = company.to_string();
    let (x, y) = tokio::join!(
        store.execute(&a, &cid, command(1, Action::Transfer { user_id: b })),
        store.execute(&a, &cid, command(1, Action::Transfer { user_id: c }))
    );
    assert_eq!([x.is_ok(), y.is_ok()].into_iter().filter(|v| *v).count(), 1);
    let owners: i64 =
        sqlx::query_scalar("SELECT count(*) FROM membership WHERE company_id=$1 AND role='owner'")
            .bind(company)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(owners, 1);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn archive_scope_binary_sources_and_secret_exclusion(p: PgPool) {
    use std::io::Read;
    let a = user(&p, "archive@example.test").await;
    let b = user(&p, "archive-other@example.test").await;
    let company:i64=sqlx::query_scalar("INSERT INTO company(creator_id,request_id,initial_settings,settings) VALUES($1,$2::text::uuid,'{}','{\"name\":\"Synthetic archive company\"}') RETURNING id").bind(a.parse::<i64>().unwrap()).bind(uuid::Uuid::new_v4().to_string()).fetch_one(&p).await.unwrap();
    sqlx::query(
        "INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'owner'),($1,$3,'viewer')",
    )
    .bind(company)
    .bind(a.parse::<i64>().unwrap())
    .bind(b.parse::<i64>().unwrap())
    .execute(&p)
    .await
    .unwrap();
    let store = PgAdministration(p.clone());
    let cid = company.to_string();
    assert!(matches!(
        store.export(&b, &cid).await,
        Err(CompanyError::Forbidden)
    ));
    assert!(matches!(
        store.export("99999999", &cid).await,
        Err(CompanyError::NotFound)
    ));
    use faktura_application::{
        sales::{self, Create, Data, Kind},
        share,
    };
    use faktura_infrastructure::{sales::PgSales, share::PgShare};
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.tax_on = None;
    let quote = sales::create(
        &PgSales(p.clone()),
        &a,
        &cid,
        Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            kind: Kind::Quote,
            data: Data {
                name: "Archive quote".into(),
                maturity_days: 14,
                status: "created".into(),
                invoice,
            },
        },
    )
    .await
    .unwrap();
    let link = share::create(
        &PgShare {
            pool: p.clone(),
            base_url: Some("https://example.test".into()),
        },
        &a,
        &cid,
        &quote.id,
        share::Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: quote.revision,
            expires_days: 30,
        },
    )
    .await
    .unwrap();
    let token = link.url.rsplit('/').next().unwrap().to_owned();
    let snapshot = serde_json::json!({"nested":[{"share":link.url,"notes":"Keep ordinary accounting notes https://example.test/manual"}]});
    sqlx::query("INSERT INTO bank_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$4)").bind(company).bind(uuid::Uuid::new_v4().to_string()).bind(a.parse::<i64>().unwrap()).bind(snapshot).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO quote_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id) VALUES($1,$2,$3,$4::text::uuid,$5,1,$5,decode('00ff','hex'),'sender@example.test','sender@example.test','customer@example.test','Synthetic outbox',$6,'archive-test')").bind(company).bind(quote.id.parse::<i64>().unwrap()).bind(a.parse::<i64>().unwrap()).bind(uuid::Uuid::new_v4().to_string()).bind(serde_json::json!({"body":format!("Link {}",link.url)})).bind(format!("Link {}",link.url)).execute(&p).await.unwrap();
    // Future tables and fields are deliberately not exported until reviewed.
    sqlx::query("CREATE TABLE archive_synthetic(company_id bigint NOT NULL, secret text)")
        .execute(&p)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE company ADD COLUMN future_secret text DEFAULT 'FUTURE-SECRET'")
        .execute(&p)
        .await
        .unwrap();
    let file = store.export(&a, &cid).await.unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(file.bytes)).unwrap();
    assert!(archive.by_name("tables/archive_synthetic.jsonl").is_err());
    let mut entire = String::new();
    for index in 0..archive.len() {
        let mut content = String::new();
        archive
            .by_index(index)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        entire.push_str(&content);
    }
    assert!(!entire.contains(&link.url));
    assert!(!entire.contains(&token));
    assert!(!entire.contains("FUTURE-SECRET"));
    assert!(entire.contains("[redacted-public-link]"));
    assert!(entire.contains("Keep ordinary accounting notes https://example.test/manual"));
    assert!(entire.contains("00ff"));
    assert!(entire.contains("Synthetic archive company"));
}

#[test]
fn administration_command_roundtrip() {
    for action in [
        Action::Leave,
        Action::Restore,
        Action::AddMember {
            email: "member@example.test".into(),
            role: Role::Viewer,
        },
    ] {
        let c = command(1, action);
        let json = serde_json::to_value(&c).unwrap();
        let roundtrip: Command = serde_json::from_value(json).unwrap();
        assert_eq!(
            serde_json::to_value(roundtrip).unwrap(),
            serde_json::to_value(c).unwrap()
        );
    }
}

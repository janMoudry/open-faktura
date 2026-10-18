use faktura_application::{auth, company, tax_classification::*};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{
    auth::PgAuth, company::PgCompany, tax_classification::PgTaxClassification,
};
use sqlx::PgPool;
const KEY: &str = "00000000-0000-4000-8000-000000000001";
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
fn command(source: &Source, revision: i64, data: Data) -> Command {
    Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        revision,
        fingerprint: source.observation.fingerprint.clone(),
        data,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn explicit_inputs_replay_bounds_profiles_and_stale_source(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let company: i64 = c.parse().unwrap();
    let actor: i64 = a.parse().unwrap();
    let invoice:i64=sqlx::query_scalar("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,number,issued_at,issue_request_id,issue_actor_id,issue_revision) VALUES($1,$2,$3,'{}','{\"issued_on\":\"2026-01-10\",\"tax_on\":\"2026-01-10\",\"calculation\":{\"tax_mode\":\"vat_payer\"}}','{\"currency\":\"CZK\",\"payable\":\"121.00\",\"amounts\":{\"net\":\"100.00\",\"vat\":\"21.00\"}}','CLASSIFY-1',now(),$4,$2,1) RETURNING id").bind(company).bind(actor).bind(uuid::Uuid::new_v4()).bind(uuid::Uuid::new_v4()).fetch_one(&p).await.unwrap();
    let received:i64=sqlx::query_scalar("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) VALUES($1,'CLASSIFY-P','{\"issued_on\":\"2026-01-10\",\"tax_on\":\"2026-01-10\",\"currency\":\"EUR\",\"exchange_rate\":{\"rate\":\"25\",\"units\":\"1\"},\"lines\":[{\"net\":\"100.00\",\"vat\":\"21.00\"}]}',121,'2026-01-11','2026-01-31') RETURNING id").bind(company).fetch_one(&p).await.unwrap();
    let store = PgTaxClassification(p.clone());
    let initial = store.summary(&a, &c, 2026).await.unwrap();
    assert_eq!(initial.unclassified_turnover, 1);
    assert_eq!(initial.unclassified_input, 1);
    assert_eq!(initial.claimed_input_vat_czk, "0.00");
    let sale = store
        .get(&a, &c, "invoice", &invoice.to_string())
        .await
        .unwrap();
    assert_eq!(sale.observation.turnover_net_czk.as_deref(), Some("100.00"));
    let input = command(
        &sale,
        0,
        Data {
            turnover_included: Some(true),
            claimed_input_vat_czk: None,
            claim_on: None,
            reason: "Explicit synthetic domestic inclusion decision".into(),
        },
    );
    let invoice_id = invoice.to_string();
    let (first, second) = tokio::join!(
        store.set(&a, &c, "invoice", &invoice_id, input.clone()),
        store.set(&a, &c, "invoice", &invoice_id, input.clone())
    );
    assert_eq!(first.unwrap().revision, 1);
    assert_eq!(second.unwrap().revision, 1);
    let purchase = store
        .get(&a, &c, "received", &received.to_string())
        .await
        .unwrap();
    assert_eq!(
        purchase.observation.recorded_vat_czk.as_deref(),
        Some("525.00")
    );
    let bad = command(
        &purchase,
        0,
        Data {
            turnover_included: None,
            claimed_input_vat_czk: Some("525.01".into()),
            claim_on: Some("2026-02-01".parse().unwrap()),
            reason: "Bad overclaim".into(),
        },
    );
    assert!(store
        .set(&a, &c, "received", &received.to_string(), bad)
        .await
        .is_err());
    let claim = command(
        &purchase,
        0,
        Data {
            turnover_included: None,
            claimed_input_vat_czk: Some("200.55".into()),
            claim_on: Some("2026-02-01".parse().unwrap()),
            reason: "Explicit synthetic partial deduction decision".into(),
        },
    );
    store
        .set(&a, &c, "received", &received.to_string(), claim.clone())
        .await
        .unwrap();
    let summary = store.summary(&a, &c, 2026).await.unwrap();
    assert_eq!(summary.included_turnover_czk, "100.00");
    assert_eq!(summary.claimed_input_vat_czk, "200.55");
    assert_eq!(summary.unclassified_input, 0);
    assert_eq!(summary.classified, 2);
    let args = serde_json::json!({"company_id":c,"year":2026});
    let via_tool = faktura_server::tax_classification::tool(
        p.clone(),
        &a,
        "tax_classification_summary",
        args.as_object().unwrap().clone(),
    )
    .await
    .unwrap();
    assert_eq!(via_tool["claimed_input_vat_czk"], "200.55");

    // Payment operations advance revision without changing the accounting snapshot.
    sqlx::query("UPDATE received_invoice SET revision=revision+1 WHERE id=$1")
        .bind(received)
        .execute(&p)
        .await
        .unwrap();
    assert!(
        store
            .get(&a, &c, "invoice", &invoice_id)
            .await
            .unwrap()
            .classification
            .unwrap()
            .current
    );
    assert!(
        store
            .get(&a, &c, "received", &received.to_string())
            .await
            .unwrap()
            .classification
            .unwrap()
            .current
    );

    assert!(store
        .get(&a, &other, "invoice", &invoice.to_string())
        .await
        .is_err());
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('tax-viewer@example.test','no-login') RETURNING id").fetch_one(&p).await.unwrap();
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(company)
        .bind(viewer)
        .execute(&p)
        .await
        .unwrap();
    assert!(store
        .get(&viewer.to_string(), &c, "invoice", &invoice.to_string())
        .await
        .is_ok());
    assert!(store
        .set(
            &viewer.to_string(),
            &c,
            "invoice",
            &invoice_id,
            input.clone()
        )
        .await
        .is_err());
    sqlx::query("UPDATE received_invoice SET revision=revision+1,data=jsonb_set(data,'{note}','\"Source changed after classification\"') WHERE id=$1").bind(received).execute(&p).await.unwrap();
    let stale = store
        .get(&a, &c, "received", &received.to_string())
        .await
        .unwrap();
    assert!(!stale.classification.as_ref().unwrap().current);
    let summary = store.summary(&a, &c, 2026).await.unwrap();
    assert_eq!(summary.outdated, 1);
    assert_eq!(summary.claimed_input_vat_czk, "0.00");
    assert_eq!(summary.unclassified_input, 1);
    let replay = store
        .set(&a, &c, "received", &received.to_string(), claim)
        .await
        .unwrap();
    assert!(!replay.current);
    let old = command(&purchase, 1, replay.data.clone());
    assert!(store
        .set(&a, &c, "received", &received.to_string(), old)
        .await
        .is_err());
    store
        .set(
            &a,
            &c,
            "received",
            &received.to_string(),
            command(&stale, 1, replay.data),
        )
        .await
        .unwrap();
    sqlx::query("UPDATE received_invoice SET cancelled=true,revision=revision+1 WHERE id=$1")
        .bind(received)
        .execute(&p)
        .await
        .unwrap();
    let summary = store.summary(&a, &c, 2026).await.unwrap();
    assert_eq!(summary.outdated, 1);
    assert_eq!(summary.claimed_input_vat_czk, "0.00");
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM tax_evidence_classification_audit")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(audit, 3);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "requires isolated PostgreSQL"]
async fn taxpayer_profile_official_codes_replay_and_isolation(p: PgPool) {
    use faktura_application::taxpayer_filing_profile::{
        self as profile, TaxpayerFilingProfileStore,
    };
    use faktura_infrastructure::taxpayer_filing_profile::PgTaxpayerFilingProfile;
    let (a, _, c, other) = setup(&p).await;
    let store = PgTaxpayerFilingProfile(p.clone());
    assert!(store.get(&a, &c).await.unwrap().data.is_none());
    let input = profile::Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        revision: 0,
        data: profile::Data {
            tax_office: "452".into(),
            tax_office_workplace: Some("2103".into()),
            taxpayer_number: "0012345678".into(),
            taxpayer_kind: "F".into(),
            vat_filer_kind: "I".into(),
        },
    };
    let (one, two) = tokio::join!(
        store.set(&a, &c, input.clone()),
        store.set(&a, &c, input.clone())
    );
    assert_eq!(one.unwrap().revision, 1);
    assert_eq!(two.unwrap().revision, 1);
    assert_eq!(
        store
            .get(&a, &c)
            .await
            .unwrap()
            .data
            .unwrap()
            .taxpayer_number,
        "0012345678"
    );
    assert!(store.get(&a, &other).await.unwrap().data.is_none());
    let outsider:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('metadata-outsider@example.test','no-login') RETURNING id").fetch_one(&p).await.unwrap();
    assert!(store.get(&outsider.to_string(), &c).await.is_err());
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(c.parse::<i64>().unwrap())
        .bind(outsider)
        .execute(&p)
        .await
        .unwrap();
    assert!(store.get(&outsider.to_string(), &c).await.is_ok());
    assert!(store
        .set(&outsider.to_string(), &c, input.clone())
        .await
        .is_err());

    let mut invalid = input.clone();
    invalid.request_id = uuid::Uuid::new_v4().to_string();
    invalid.revision = 1;
    invalid.data.tax_office = "451".into();
    assert!(store.set(&a, &c, invalid).await.is_err());
    let mut changed = input.clone();
    changed.data.taxpayer_kind = "P".into();
    assert!(store.set(&a, &c, changed).await.is_err());
    let via_tool = faktura_server::taxpayer_filing_profile::tool(
        p.clone(),
        &a,
        "taxpayer_filing_profile_get",
        serde_json::json!({"company_id":c})
            .as_object()
            .unwrap()
            .clone(),
    )
    .await
    .unwrap();
    assert_eq!(via_tool["data"]["vat_filer_kind"], "I");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM taxpayer_filing_profile_audit")
            .fetch_one(&p)
            .await
            .unwrap(),
        1
    );
    for code in ["999", "000", "abc"] {
        let mut bad = input.clone();
        bad.data.tax_office = code.into();
        assert!(profile::validate(&bad).is_err());
    }
}

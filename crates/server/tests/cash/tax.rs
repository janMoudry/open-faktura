use super::*;
use faktura_application::cash::{TaxInput, TaxRequest};
use faktura_domain::invoice::PriceMode;
fn tax() -> TaxInput {
    TaxInput {
        price_mode: PriceMode::Net,
        vat_percent: "21".into(),
        tax_on: "2026-10-01".parse().unwrap(),
    }
}
async fn payer(p: &PgPool, c: &str) {
    sqlx::query("UPDATE company SET settings=settings || '{\"tax_mode\":\"vat_payer\",\"tax_effective_from\":\"2026-01-01\",\"name\":\"Synthetic VAT company\",\"vat_number\":\"CZ12345678\"}'::jsonb WHERE id=$1::text::bigint").bind(c).execute(p).await.unwrap();
}
fn cmd(ac: &Account, value: &str) -> Command {
    Command {
        request_id: key(),
        action: Action::TaxManual {
            account_id: ac.id.clone(),
            tax: tax(),
            data: MovementData {
                booked_on: "2026-10-01".parse().unwrap(),
                amount: value.into(),
                currency: "CZK".into(),
                variable_symbol: "".into(),
                counterparty: "Synthetic partner".into(),
                description: "Standalone service".into(),
                person: "Cashier".into(),
            },
        },
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_prices_scope_replay_snapshot_cancel_and_pdf(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    assert!(s.execute(&a, &c, cmd(&ac, "100")).await.is_err());
    payer(&p, &c).await;
    let request = cmd(&ac, "100");
    let retry = serde_json::to_value(&request).unwrap();
    let (first, second) = tokio::join!(
        s.execute(&a, &c, request),
        s.execute(&a, &c, serde_json::from_value(retry.clone()).unwrap())
    );
    let Reply::Movement(m) = first.unwrap() else {
        panic!()
    };
    assert_eq!(
        serde_json::to_value(second.unwrap()).unwrap()["value"]["id"],
        m.id
    );
    assert_eq!(m.data.amount, "121.00");
    assert_eq!(
        s.movements(
            &a,
            &c,
            cash::Search {
                status: "tax".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        s.movements(
            &a,
            &c,
            cash::Search {
                status: "unmatched".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    assert_eq!(m.tax.as_ref().unwrap().amounts.vat, "21.00");
    assert!(s.candidates(&a, &c, &m.id, "").await.unwrap().is_empty());
    assert!(s
        .execute(
            &a,
            &c,
            Command {
                request_id: key(),
                action: Action::Match {
                    movement_id: m.id.clone(),
                    revision: m.revision,
                    target_kind: "invoice".into(),
                    document_id: "1".into(),
                    target_revision: 1,
                    payment_id: None
                }
            }
        )
        .await
        .is_err());
    assert!(s.get(&a, &other, &m.id).await.is_err());
    let foreign = account(&s, &a, &other).await;
    assert!(s.execute(&a, &c, cmd(&foreign, "100")).await.is_err());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        1
    );
    let balance = s
        .balance(
            &a,
            &c,
            cash::Search {
                account_id: ac.id.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(balance.closing, "1121.00");
    let Reply::Movement(expense) = s.execute(&a, &c, cmd(&ac, "-100")).await.unwrap() else {
        panic!()
    };
    assert_eq!(expense.data.amount, "-121.00");
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{tax_mode}',to_jsonb('non_payer'::text)) WHERE id=$1::text::bigint").bind(&c).execute(&p).await.unwrap();
    assert_eq!(
        s.get(&a, &c, &m.id).await.unwrap().tax.unwrap().amounts.vat,
        "21.00"
    );
    assert!(s
        .execute(&a, &c, serde_json::from_value(retry).unwrap())
        .await
        .is_ok());
    for (id, name, lang) in [
        (&m.id, "cash-tax-cs", "cs"),
        (&expense.id, "cash-tax-en", "en"),
    ] {
        let pdf = s.pdf(&a, &c, id, lang).await.unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        if let Ok(dir) = std::env::var("CASH_PDF_TEST_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/{name}.pdf"), pdf).unwrap();
        }
    }
    run(
        &s,
        &a,
        &c,
        Action::Cancel {
            movement_id: m.id.clone(),
            revision: m.revision,
            reason: "Synthetic tax cancellation".into(),
        },
    )
    .await;
    assert_eq!(
        s.balance(
            &a,
            &c,
            cash::Search {
                account_id: ac.id.clone(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .closing,
        "879.00"
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.execute(&a, &c, cmd(&ac, "10")).await.is_err());
    assert!(s.pdf(&a, &c, &m.id, "cs").await.is_ok());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_preview_validation_and_failed_insert_rollback(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    payer(&p, &c).await;
    let preview = s
        .tax_preview(
            &a,
            &c,
            TaxRequest {
                amount: "100".into(),
                tax: tax(),
            },
        )
        .await
        .unwrap();
    assert_eq!(preview.amounts.gross, "121.00");
    for (field, value) in [
        ("tax_on", "2025-12-31"),
        ("tax_on", "9999-12-31"),
        ("vat_percent", "19"),
    ] {
        let mut v = serde_json::to_value(tax()).unwrap();
        v[field] = value.into();
        assert!(s
            .tax_preview(
                &a,
                &c,
                TaxRequest {
                    amount: "100".into(),
                    tax: serde_json::from_value(v).unwrap()
                }
            )
            .await
            .is_err());
    }
    let mut invalid = cmd(&ac, "100");
    if let Action::TaxManual { data, .. } = &mut invalid.action {
        data.description.clear();
    }
    assert!(s.execute(&a, &c, invalid).await.is_err());
    // Fail after movement/number allocation; the full command must roll back.
    sqlx::query("CREATE FUNCTION fail_cash_tax() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.tax IS NOT NULL THEN RAISE EXCEPTION 'synthetic failure'; END IF; RETURN NEW; END $$").execute(&p).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_cash_tax BEFORE UPDATE ON cash_movement FOR EACH ROW EXECUTE FUNCTION fail_cash_tax()").execute(&p).await.unwrap();
    let request = cmd(&ac, "100");
    let retry = serde_json::to_value(&request).unwrap();
    assert!(s.execute(&a, &c, request).await.is_err());
    assert_eq!(
        s.movements(&a, &c, Default::default()).await.unwrap().total,
        0
    );
    sqlx::query("DROP TRIGGER fail_cash_tax ON cash_movement")
        .execute(&p)
        .await
        .unwrap();
    let Reply::Movement(m) = s
        .execute(&a, &c, serde_json::from_value(retry).unwrap())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(m.number, "PP2026000001");
}

#[sqlx::test(migrations = false)]
#[ignore = "isolated PostgreSQL upgrade"]
async fn upgrade_030_preserves_ledger_and_cached_pdf(p: PgPool) {
    let old = sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            sqlx::migrate!("../../migrations")
                .iter()
                .filter(|m| m.version < 202610140031)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    };
    old.run(&p).await.unwrap();
    let (a, _, c, _) = setup(&p).await;
    let s = PgCash(p.clone());
    let ac = account(&s, &a, &c).await;
    let request = cmd(&ac, "100");
    let Action::TaxManual { data, .. } = request.action else {
        panic!()
    };
    let id:i64=sqlx::query_scalar("INSERT INTO cash_movement(company_id,account_id,number,data,amount,booked_on,issuer) VALUES($1::text::bigint,$2::text::bigint,'PP2026000001',$3,100,'2026-10-01',(SELECT settings FROM company WHERE id=$1::text::bigint)) RETURNING id").bind(&c).bind(&ac.id).bind(serde_json::to_value(data).unwrap()).fetch_one(&p).await.unwrap();
    let cached = b"%PDF-synthetic-cache-preservation";
    sqlx::query("INSERT INTO cash_pdf(company_id,movement_id,revision,language,content) VALUES($1::text::bigint,$2,1,'cs',$3)").bind(&c).bind(id).bind(cached.as_slice()).execute(&p).await.unwrap();
    sqlx::migrate!("../../migrations").run(&p).await.unwrap();
    let m = s.get(&a, &c, &id.to_string()).await.unwrap();
    assert!(m.tax.is_none());
    assert_eq!(m.data.amount, "100");
    assert_eq!(s.pdf(&a, &c, &id.to_string(), "cs").await.unwrap(), cached);
    assert_eq!(
        s.balance(
            &a,
            &c,
            cash::Search {
                account_id: ac.id,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .closing,
        "1100.00"
    );
}

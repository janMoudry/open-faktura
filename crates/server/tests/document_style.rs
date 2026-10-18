#[path = "support/pdf_text.rs"]
mod pdf_text;
use faktura_application::{
    company,
    company::CompanyError,
    document_style::*,
    draft::{self, DraftStore},
};
use faktura_domain::{
    company::CompanySettings,
    invoice::{DraftData, TaxMode},
};
use faktura_infrastructure::{company::PgCompany, document_style::PgStyle, draft::PgDraft};
use sqlx::PgPool;
fn command(revision: i64, data: Style) -> Command {
    Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        revision,
        data,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn brand_revision_image_limits_roles_and_immutable_pdf(p: PgPool) {
    use base64::Engine;
    let actor:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('style@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = actor.to_string();
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('style-view@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let c = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO membership(company_id,user_id,role) VALUES($1::text::bigint,$2,'viewer')",
    )
    .bind(&c.id)
    .bind(viewer)
    .execute(&p)
    .await
    .unwrap();
    let styles = PgStyle(p.clone());
    let drafts = PgDraft(p.clone());
    assert_eq!(styles.get(&actor, &c.id).await.unwrap().revision, 0);
    let mut style = Style {
        delivery_note_prices: false,
        before_items_cs: "Původní český úvod".into(),
        before_items_en: "Original English introduction".into(),
        logo_base64: Some(
            base64::engine::general_purpose::STANDARD
                .encode(include_bytes!("../../../tests/fixtures/document-logo.png")),
        ),
        signature_base64: None,
        footer_cs: "Původní české zápatí".into(),
        footer_en: "Original English footer".into(),
    };
    assert!(matches!(
        styles
            .set(&viewer.to_string(), &c.id, command(0, style.clone()))
            .await,
        Err(CompanyError::Forbidden)
    ));
    let set = command(0, style.clone());
    assert_eq!(
        styles
            .set(&actor, &c.id, set.clone())
            .await
            .unwrap()
            .revision,
        1
    );
    assert_eq!(styles.set(&actor, &c.id, set).await.unwrap().revision, 1);
    let mut bad = style.clone();
    bad.logo_base64 = Some("PHN2Zz48L3N2Zz4=".into());
    assert!(matches!(
        styles.set(&actor, &c.id, command(1, bad)).await,
        Err(CompanyError::Invalid(_))
    ));
    for invalid in ["x".repeat(1001), "invalid\0control".to_owned()] {
        let mut bad = style.clone();
        bad.before_items_en = invalid;
        assert!(matches!(
            styles.set(&actor, &c.id, command(1, bad)).await,
            Err(CompanyError::Invalid(_))
        ));
    }
    let mut data: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    let today: sqlx::types::chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&p)
            .await
            .unwrap();
    data.issued_on = Some(today);
    data.due_on = Some(today);
    data.tax_on = None;
    data.supplier.registration_number = "12345678".into();
    data.payment.method = "cash".into();
    data.calculation.tax_mode = TaxMode::NonPayer;
    data.calculation.lines[0].vat_percent = "0".into();
    data.language = "en".into();
    sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{number_year}',$2::text::jsonb) WHERE id=$1::text::bigint").bind(&c.id).bind(today.format("%Y").to_string()).execute(&p).await.unwrap();
    let d = draft::create(
        &drafts,
        &actor,
        &c.id,
        draft::Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            data,
        },
    )
    .await
    .unwrap();
    draft::issue(
        &drafts,
        &actor,
        &c.id,
        &d.id,
        draft::Issue {
            request_id: uuid::Uuid::new_v4().to_string(),
            revision: d.revision,
        },
    )
    .await
    .unwrap();
    style.footer_en = "Changed English footer".into();
    style.before_items_en = "Changed English introduction".into();
    styles.set(&actor, &c.id, command(1, style)).await.unwrap();
    let pdf = drafts.pdf(&actor, &c.id, &d.id).await.unwrap();
    let text = pdf_text::text(&pdf);
    assert!(text.contains("Original English footer"));
    assert!(text.contains("Original English introduction"));
    assert!(!text.contains("Changed English introduction"));
    assert!(
        text.find("Original English introduction").unwrap()
            < text.find("Synthetic service").unwrap()
    );
    assert!(!text.contains("Changed English footer"));
    assert!(!text.contains("Původní"));
    let cached = drafts.pdf(&actor, &c.id, &d.id).await.unwrap();
    assert_eq!(pdf, cached);
    let blocked=sqlx::query("UPDATE invoice_draft SET document_style='{}' WHERE company_id=$1::text::bigint AND id=$2::text::bigint").bind(&c.id).bind(&d.id).execute(&p).await;
    assert!(blocked.is_err());
    if let Ok(dir) = std::env::var("OUTPUT_REVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/branded-invoice.pdf"), pdf).unwrap();
    }
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn quote_advance_cash_preserve_saved_style(p: PgPool) {
    use faktura_application::{
        advance::AdvanceStore,
        cash::{Action, CashStore, Command as CashCommand, Reply},
        delivery::DeliveryStore,
        sales::{self, Create, Data, Kind},
    };
    use faktura_infrastructure::{
        advance::PgAdvance, cash::PgCash, delivery::PgDelivery, sales::PgSales,
    };
    let actor:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('style-shared@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = actor.to_string();
    let c = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let styles = PgStyle(p.clone());
    let mut style = Style {
        delivery_note_prices: false,
        before_items_cs: "Původní český úvod".into(),
        before_items_en: "Original English introduction".into(),
        logo_base64: None,
        signature_base64: None,
        footer_cs: "Původní společné zápatí".into(),
        footer_en: "Original shared footer".into(),
    };
    styles
        .set(&actor, &c.id, command(0, style.clone()))
        .await
        .unwrap();
    let mut invoice: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.calculation.tax_mode = TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.tax_on = None;
    invoice.language = "en".into();
    let data = Data {
        name: "Shared brand quotation".into(),
        maturity_days: 14,
        status: "created".into(),
        invoice,
    };
    let quote = sales::create(
        &PgSales(p.clone()),
        &actor,
        &c.id,
        Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            kind: Kind::Quote,
            data: data.clone(),
        },
    )
    .await
    .unwrap();
    let advance = sales::create(
        &PgSales(p.clone()),
        &actor,
        &c.id,
        Create {
            request_id: uuid::Uuid::new_v4().to_string(),
            kind: Kind::Advance,
            data: data.clone(),
        },
    )
    .await
    .unwrap();
    let cash_store = PgCash(p.clone());
    let account = match cash_store
        .execute(
            &actor,
            &c.id,
            CashCommand {
                request_id: uuid::Uuid::new_v4().to_string(),
                action: Action::Account {
                    id: None,
                    revision: None,
                    archived: false,
                    data: faktura_domain::cash::AccountData {
                        name: "Synthetic style account".into(),
                        currency: "CZK".into(),
                        opening_on: "2026-01-01".parse().unwrap(),
                        opening_balance: "0".into(),
                    },
                },
            },
        )
        .await
        .unwrap()
    {
        Reply::Account(v) => v,
        _ => panic!(),
    };
    let movement = match cash_store
        .execute(
            &actor,
            &c.id,
            CashCommand {
                request_id: uuid::Uuid::new_v4().to_string(),
                action: Action::Manual {
                    account_id: account.id,
                    data: faktura_domain::cash::MovementData {
                        booked_on: "2026-10-18".parse().unwrap(),
                        amount: "123".into(),
                        currency: "CZK".into(),
                        variable_symbol: "123".into(),
                        counterparty: "Synthetic customer".into(),
                        description: "Style test".into(),
                        person: "Cashier".into(),
                    },
                },
            },
        )
        .await
        .unwrap()
    {
        Reply::Movement(v) => v,
        _ => panic!(),
    };
    style.footer_en = "Changed shared footer".into();
    styles.set(&actor, &c.id, command(1, style)).await.unwrap();
    let quote_pdf = PgDelivery {
        pool: p.clone(),
        mail: None,
    }
    .pdf(&actor, &c.id, &quote.id, None)
    .await
    .unwrap();
    let advance_pdf = PgAdvance(p.clone())
        .pdf(&actor, &c.id, &advance.id)
        .await
        .unwrap();
    let cash_pdf = cash_store
        .pdf(&actor, &c.id, &movement.id, "en")
        .await
        .unwrap();
    for pdf in [&quote_pdf, &advance_pdf, &cash_pdf] {
        let text = pdf_text::text(pdf);
        assert!(text.contains("Original shared footer"));
        assert!(!text.contains("Changed shared footer"));
    }
    for pdf in [&quote_pdf, &advance_pdf] {
        assert!(pdf_text::text(pdf).contains("Original English introduction"));
    }
    assert!(!pdf_text::text(&cash_pdf).contains("Original English introduction"));
    assert_eq!(
        cash_pdf,
        cash_store
            .pdf(&actor, &c.id, &movement.id, "en")
            .await
            .unwrap()
    );
    if let Ok(dir) = std::env::var("OUTPUT_REVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        for (name, pdf) in [
            ("branded-quote", quote_pdf),
            ("branded-advance", advance_pdf),
            ("branded-cash", cash_pdf),
        ] {
            std::fs::write(format!("{dir}/{name}.pdf"), pdf).unwrap();
        }
    }
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tax_receipt_credit_new_brand_and_legacy_pdf_history(p: PgPool) {
    use faktura_application::{
        advance::{self, AdvanceStore},
        credit_note::{self, Action as CreditAction, Command as CreditCommand, CreditStore},
        sales::{self, SalesStore},
        tax_receipt::{self, TaxReceiptStore},
    };
    use faktura_infrastructure::{
        advance::PgAdvance, credit_note::PgCredit, sales::PgSales, tax_receipt::PgTaxReceipt,
    };
    let actor:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('style-tax-credit@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = actor.to_string();
    let today: sqlx::types::chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&p)
            .await
            .unwrap();
    let c = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings {
            number_year: today.format("%Y").to_string().parse().unwrap(),
            tax_mode: "vat_payer".into(),
            vat_number: "CZ12345678".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let styles = PgStyle(p.clone());
    let drafts = PgDraft(p.clone());
    let tax = PgTaxReceipt(p.clone());
    let credits = PgCredit(p.clone());
    let mut saved = Vec::new();
    for branded in [false, true] {
        if branded {
            styles
                .set(
                    &actor,
                    &c.id,
                    command(
                        0,
                        Style {
                            delivery_note_prices: false,
                            before_items_cs: "Původní český úvod".into(),
                            before_items_en: "Original English introduction".into(),
                            logo_base64: None,
                            signature_base64: None,
                            footer_cs: "Původní daňové zápatí".into(),
                            footer_en: "Original tax document footer".into(),
                        },
                    ),
                )
                .await
                .unwrap();
        }
        let mut data: DraftData =
            serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap();
        data.issued_on = Some(today);
        data.due_on = Some(today);
        data.tax_on = Some(today);
        data.supplier.registration_number = "12345678".into();
        data.supplier.vat_number = "CZ12345678".into();
        data.payment.method = "cash".into();
        data.calculation.lines[0].quantity = "1".into();
        data.calculation.lines[0].unit_price = "1000".into();
        data.calculation.lines[0].vat_percent = "21".into();
        data.language = "en".into();
        let draft = draft::create(
            &drafts,
            &actor,
            &c.id,
            draft::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data: data.clone(),
            },
        )
        .await
        .unwrap();
        let invoice = draft::issue(
            &drafts,
            &actor,
            &c.id,
            &draft.id,
            draft::Issue {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: draft.revision,
            },
        )
        .await
        .unwrap();
        let original_pdf = drafts.pdf(&actor, &c.id, &invoice.id).await.unwrap();
        data.tax_on = None;
        let advance = sales::create(
            &PgSales(p.clone()),
            &actor,
            &c.id,
            sales::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                kind: sales::Kind::Advance,
                data: sales::Data {
                    name: "Tax style advance".into(),
                    maturity_days: 14,
                    status: "created".into(),
                    invoice: data,
                },
            },
        )
        .await
        .unwrap();
        advance::register(
            &PgAdvance(p.clone()),
            &actor,
            &c.id,
            &advance.id,
            advance::Register {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: advance.revision,
                payment: faktura_domain::payment::PaymentInput {
                    amount: "1210".into(),
                    paid_on: today,
                    method: "bank_transfer".into(),
                    note: "Synthetic deposit".into(),
                },
            },
        )
        .await
        .unwrap();
        let payment = PgAdvance(p.clone())
            .payments(&actor, &c.id, &advance.id)
            .await
            .unwrap()
            .remove(0);
        let current = PgSales(p.clone())
            .get(&actor, &c.id, &advance.id)
            .await
            .unwrap();
        let receipt = tax_receipt::issue(
            &tax,
            &actor,
            &c.id,
            tax_receipt::Issue {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: current.revision,
                advance_id: advance.id,
                payment_id: payment.id,
                issued_on: today,
                description: "Synthetic deposit payment".into(),
                language: "en".into(),
            },
        )
        .await
        .unwrap();
        let tax_pdf = tax.pdf(&actor, &c.id, &receipt.id).await.unwrap();
        let mut credit = credit_note::create(
            &credits,
            &actor,
            &c.id,
            credit_note::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                invoice_id: invoice.id.clone(),
            },
        )
        .await
        .unwrap();
        credit.data.reason = "Synthetic correction".into();
        credit = credit_note::command(
            &credits,
            &actor,
            &c.id,
            &credit.id,
            CreditCommand {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: credit.revision,
                action: CreditAction::Update {
                    data: credit.data.clone(),
                },
            },
        )
        .await
        .unwrap();
        let credit = credit_note::command(
            &credits,
            &actor,
            &c.id,
            &credit.id,
            CreditCommand {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: credit.revision,
                action: CreditAction::Issue,
            },
        )
        .await
        .unwrap();
        let credit_pdf = credits.pdf(&actor, &c.id, &credit.id).await.unwrap();
        for bytes in [&tax_pdf, &credit_pdf] {
            assert_eq!(
                pdf_text::text(bytes).contains("Original tax document footer"),
                branded
            );
            assert_eq!(
                pdf_text::text(bytes).contains("Original English introduction"),
                branded
            );
        }
        saved.push((
            branded,
            invoice.id,
            receipt.id,
            credit.id,
            original_pdf,
            tax_pdf,
            credit_pdf,
        ));
    }
    styles
        .set(
            &actor,
            &c.id,
            command(
                1,
                Style {
                    delivery_note_prices: false,
                    before_items_cs: "Původní český úvod".into(),
                    before_items_en: "Original English introduction".into(),
                    logo_base64: None,
                    signature_base64: None,
                    footer_cs: "Změněné".into(),
                    footer_en: "Changed tax document footer".into(),
                },
            ),
        )
        .await
        .unwrap();
    for (branded, invoice, tax_id, credit_id, original, tax_pdf, credit_pdf) in saved {
        assert_eq!(original, drafts.pdf(&actor, &c.id, &invoice).await.unwrap());
        assert_eq!(tax_pdf, tax.pdf(&actor, &c.id, &tax_id).await.unwrap());
        assert_eq!(
            credit_pdf,
            credits.pdf(&actor, &c.id, &credit_id).await.unwrap()
        );
        if branded {
            if let Ok(dir) = std::env::var("OUTPUT_REVIEW_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(format!("{dir}/branded-tax-receipt.pdf"), tax_pdf).unwrap();
                std::fs::write(format!("{dir}/branded-credit-note.pdf"), credit_pdf).unwrap();
            }
        }
    }
}

#[test]
fn legacy_style_roundtrips_without_new_empty_fields() {
    let legacy = serde_json::json!({"logo_base64":null,"signature_base64":null,"footer_cs":"","footer_en":""});
    let style: Style = serde_json::from_value(legacy.clone()).unwrap();
    assert!(style.before_items_cs.is_empty());
    assert!(style.before_items_en.is_empty());
    assert_eq!(serde_json::to_value(style).unwrap(), legacy);
}

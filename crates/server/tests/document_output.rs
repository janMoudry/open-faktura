#[path = "support/pdf_text.rs"]
mod pdf_text;
use faktura_application::{
    company,
    company::CompanyError,
    document_output::*,
    draft::{self},
};
use faktura_domain::{
    company::CompanySettings,
    invoice::{DraftData, TaxMode},
};
use faktura_infrastructure::{company::PgCompany, document_output::PgOutput, draft::PgDraft};
use sqlx::PgPool;
fn request(ids: Vec<String>, kind: Kind, language: &str) -> Request {
    Request {
        delivery_note_prices: None,
        ids,
        kind,
        language: language.into(),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn outputs_snapshot_scope_batch_pages_and_reversed_payments(p: PgPool) {
    let actor:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('output@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = actor.to_string();
    let c = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let other = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let store = PgDraft(p.clone());
    let output = PgOutput(p.clone());
    let mut ids = Vec::new();
    for n in 0..2 {
        let mut data: DraftData =
            serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap();
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
        data.customer.as_mut().unwrap().name = format!("Customer {n}");
        sqlx::query("UPDATE company SET settings=jsonb_set(settings,'{number_year}',$2::text::jsonb) WHERE id=$1::text::bigint").bind(&c.id).bind(today.format("%Y").to_string()).execute(&p).await.unwrap();
        let d = draft::create(
            &store,
            &actor,
            &c.id,
            draft::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data,
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            output
                .render(
                    &actor,
                    &c.id,
                    request(vec![d.id.clone()], Kind::DeliveryNote, "cs")
                )
                .await,
            Err(CompanyError::NotFound)
        ));
        let issued = draft::issue(
            &store,
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
        ids.push(issued.id);
    }
    assert!(output
        .render(&actor, &other.id, request(ids.clone(), Kind::BulkPdf, "cs"))
        .await
        .is_err());
    let batch = output
        .render(&actor, &c.id, request(ids.clone(), Kind::BulkPdf, "cs"))
        .await
        .unwrap();
    if let Ok(dir) = std::env::var("OUTPUT_REVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/batch.pdf"), &batch.bytes).unwrap();
    }
    let parsed = lopdf::Document::load_mem(&batch.bytes).unwrap();
    assert_eq!(parsed.get_pages().len(), 2);
    let text = pdf_text::text(&batch.bytes);
    assert_eq!(text.matches("Customer 0").count(), 1, "{text:?}");
    assert_eq!(text.matches("Customer 1").count(), 1);
    assert!(output
        .render(
            &actor,
            &c.id,
            request(vec![ids[0].clone(), ids[0].clone()], Kind::BulkPdf, "en")
        )
        .await
        .is_err());
    let rtf = output
        .render(
            &actor,
            &c.id,
            request(vec![ids[0].clone()], Kind::Rtf, "en"),
        )
        .await
        .unwrap();
    assert!(String::from_utf8(rtf.bytes).unwrap().contains("Customer 0"));
    let note = output
        .render(
            &actor,
            &c.id,
            request(vec![ids[0].clone()], Kind::DeliveryNote, "en"),
        )
        .await
        .unwrap();
    let text = pdf_text::text(&note.bytes);
    assert!(text.contains("Delivery note"));
    assert!(!text.contains("Payable"));
    assert!(matches!(
        output
            .render(
                &actor,
                &c.id,
                request(vec![ids[0].clone()], Kind::PaymentConfirmation, "en")
            )
            .await,
        Err(CompanyError::Conflict)
    ));
    // Saved print default and explicit override apply to auxiliary output only.
    use faktura_application::{
        document_style::{Command as StyleCommand, Style, StyleStore},
        draft::DraftStore,
    };
    use faktura_infrastructure::document_style::PgStyle;
    let original = store.pdf(&actor, &c.id, &ids[0]).await.unwrap();
    let styles = PgStyle(p.clone());
    styles
        .set(
            &actor,
            &c.id,
            StyleCommand {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: 0,
                data: Style {
                    delivery_note_prices: true,
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
    assert!(
        styles
            .get(&actor, &c.id)
            .await
            .unwrap()
            .data
            .delivery_note_prices
    );
    assert!(
        !styles
            .get(&actor, &other.id)
            .await
            .unwrap()
            .data
            .delivery_note_prices
    );
    for language in ["cs", "en"] {
        let priced = output
            .render(
                &actor,
                &c.id,
                request(vec![ids[0].clone()], Kind::DeliveryNote, language),
            )
            .await
            .unwrap();
        let text = pdf_text::text(&priced.bytes);
        assert!(
            text.contains(if language == "cs" {
                "Celkem položka"
            } else {
                "Line gross"
            }),
            "{text}"
        );
        let amounts: serde_json::Value =
            sqlx::query_scalar("SELECT totals FROM invoice_draft WHERE id=$1::text::bigint")
                .bind(&ids[0])
                .fetch_one(&p)
                .await
                .unwrap();
        let gross = amounts["lines"][0]["amounts"]["gross"].as_str().unwrap();
        assert!(text.contains(&if language == "cs" {
            gross.replace('.', ",")
        } else {
            gross.to_owned()
        }));
        assert!(!text.contains("Payable"));
        let mut without = request(vec![ids[0].clone()], Kind::DeliveryNote, language);
        without.delivery_note_prices = Some(false);
        let no_prices = output.render(&actor, &c.id, without).await.unwrap();
        let text = pdf_text::text(&no_prices.bytes);
        assert!(!text.contains("CZK"));
        assert!(!text.contains("Line gross"));
        assert!(!text.contains("Celkem položka"));
        if let Ok(dir) = std::env::var("OUTPUT_REVIEW_DIR") {
            std::fs::write(
                format!("{dir}/delivery-note-prices-{language}.pdf"),
                priced.bytes,
            )
            .unwrap();
        }
    }
    styles
        .set(
            &actor,
            &c.id,
            StyleCommand {
                request_id: uuid::Uuid::new_v4().to_string(),
                revision: 1,
                data: Style::default(),
            },
        )
        .await
        .unwrap();
    let mut explicit = request(vec![ids[0].clone()], Kind::DeliveryNote, "en");
    explicit.delivery_note_prices = Some(true);
    assert!(
        pdf_text::text(&output.render(&actor, &c.id, explicit).await.unwrap().bytes)
            .contains("Line gross")
    );
    assert_eq!(store.pdf(&actor, &c.id, &ids[0]).await.unwrap(), original);
    let mut invalid = request(vec![ids[0].clone()], Kind::Rtf, "en");
    invalid.delivery_note_prices = Some(true);
    assert!(matches!(
        output.render(&actor, &c.id, invalid).await,
        Err(CompanyError::Invalid(_))
    ));
    let payment:i64=sqlx::query_scalar("INSERT INTO invoice_payment(company_id,invoice_id,amount,paid_on,method,note,actor_id) VALUES($1::text::bigint,$2::text::bigint,1.23,current_date,'bank_transfer','synthetic',$3::text::bigint) RETURNING id").bind(&c.id).bind(&ids[0]).bind(&actor).fetch_one(&p).await.unwrap();
    let confirmation = output
        .render(
            &actor,
            &c.id,
            request(vec![ids[0].clone()], Kind::PaymentConfirmation, "en"),
        )
        .await
        .unwrap();
    let text = pdf_text::text(&confirmation.bytes);
    assert!(text.contains("1.23 CZK"));
    assert!(text.contains("Bank transfer"));
    sqlx::query("INSERT INTO invoice_payment_reversal(company_id,invoice_id,payment_id,reason,actor_id) VALUES($1::text::bigint,$2::text::bigint,$3,'Synthetic reversal',$4::text::bigint)").bind(&c.id).bind(&ids[0]).bind(payment).bind(&actor).execute(&p).await.unwrap();
    assert!(matches!(
        output
            .render(
                &actor,
                &c.id,
                request(vec![ids[0].clone()], Kind::PaymentConfirmation, "en")
            )
            .await,
        Err(CompanyError::Conflict)
    ));
    // Persist only synthetic artifacts for visual review when explicitly requested by the test runner.
    if let Ok(dir) = std::env::var("OUTPUT_REVIEW_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/batch.pdf"), batch.bytes).unwrap();
        std::fs::write(format!("{dir}/delivery-note.pdf"), note.bytes).unwrap();
        std::fs::write(
            format!("{dir}/payment-confirmation.pdf"),
            confirmation.bytes,
        )
        .unwrap();
    }
}

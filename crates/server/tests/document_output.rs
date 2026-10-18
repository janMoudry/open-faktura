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

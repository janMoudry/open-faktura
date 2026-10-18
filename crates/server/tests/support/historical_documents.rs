//! Explicit pre-upgrade fixtures. Never call today's issuer/style-aware sales
//! store against an old schema: these helpers model rows written by that release.
#![allow(dead_code)]
use faktura_application::{draft, sales};
use faktura_infrastructure::draft::PgDraft;
use sqlx::PgPool;

pub async fn invoice(
    pool: &PgPool,
    actor: &str,
    company: &str,
    command: draft::Create,
) -> (draft::Draft, Vec<u8>) {
    let request = command.request_id.clone();
    let mut value = draft::create(&PgDraft(pool.clone()), actor, company, command)
        .await
        .unwrap();
    value.number = Some(format!("LEGACY-{}", value.id));
    value.issued_at = Some(sqlx::types::chrono::Utc::now().to_rfc3339());
    value.revision += 1;
    let pdf = faktura_infrastructure::invoice_pdf::render(&value).unwrap();
    sqlx::query("UPDATE invoice_draft SET number=$3,issued_at=now(),issue_request_id=$4::text::uuid,issue_actor_id=$5::text::bigint,issue_revision=revision,revision=revision+1,pdf=$6 WHERE company_id=$1::text::bigint AND id=$2::text::bigint")
        .bind(company).bind(&value.id).bind(&value.number).bind(request).bind(actor).bind(&pdf).execute(pool).await.unwrap();
    (value, pdf)
}

pub async fn sales(
    pool: &PgPool,
    actor: &str,
    company: &str,
    command: sales::Create,
) -> sales::Document {
    let kind = match command.kind {
        sales::Kind::Quote => "quote",
        sales::Kind::Template => "template",
        sales::Kind::Advance => "advance",
    };
    let data = serde_json::to_value(&command.data).unwrap();
    let number =
        (command.kind != sales::Kind::Template).then(|| format!("LEGACY-{}", command.request_id));
    let id: i64 = sqlx::query_scalar("INSERT INTO sales_document(company_id,creator_id,request_id,kind,initial_data,data,number) VALUES($1::text::bigint,$2::text::bigint,$3::text::uuid,$4,$5,$5,$6) RETURNING id")
        .bind(company).bind(actor).bind(command.request_id).bind(kind).bind(data).bind(&number).fetch_one(pool).await.unwrap();
    let totals = faktura_domain::invoice::calculate(&command.data.invoice.calculation).ok();
    let value = sales::Document {
        id: id.to_string(),
        company_id: company.into(),
        kind: command.kind,
        revision: 1,
        archived: false,
        number,
        invoice_id: None,
        data: command.data,
        totals,
        advance_paid: (kind == "advance").then(|| "0".into()),
        document_style: None,
    };
    if kind == "advance" {
        let pdf = faktura_infrastructure::invoice_pdf::render_advance(&draft::Draft {
            id: value.id.clone(),
            company_id: value.company_id.clone(),
            revision: value.revision,
            data: value.data.invoice.clone(),
            totals: value.totals.clone(),
            number: value.number.clone(),
            settlement: None,
            issued_at: None,
            advances: None,
        })
        .unwrap();
        sqlx::query("UPDATE sales_document SET advance_totals=$2,advance_pdf=$3 WHERE id=$1")
            .bind(id)
            .bind(serde_json::to_value(&value.totals).unwrap())
            .bind(pdf)
            .execute(pool)
            .await
            .unwrap();
    }
    value
}

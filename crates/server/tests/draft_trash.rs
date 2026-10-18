use faktura_application::{
    company,
    company::CompanyError,
    draft::{self},
    draft_trash::*,
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{company::PgCompany, draft::PgDraft, draft_trash::PgTrash};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn trash_restore_revision_replay_roles_and_issue_guard(p: PgPool) {
    let a:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('trash@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let a = a.to_string();
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('trash-viewer@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let companies = PgCompany(p.clone());
    let c = company::create(&companies, &a, &key(), CompanySettings::default())
        .await
        .unwrap();
    let other = company::create(&companies, &a, &key(), CompanySettings::default())
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
    let ds = PgDraft(p.clone());
    let ts = PgTrash(p.clone());
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    data.supplier.registration_number = "12345678".into();
    data.payment.method = "cash".into();
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    data.calculation.lines[0].vat_percent = "0".into();
    data.tax_on = None;
    let d = draft::create(
        &ds,
        &a,
        &c.id,
        draft::Create {
            request_id: key(),
            data,
        },
    )
    .await
    .unwrap();
    let cmd = Command {
        request_id: key(),
        id: d.id.clone(),
        revision: d.revision,
        action: Action::Trash,
    };
    assert!(matches!(
        ts.execute(&viewer.to_string(), &c.id, cmd.clone()).await,
        Err(CompanyError::Forbidden)
    ));
    assert!(ts.execute(&a, &other.id, cmd.clone()).await.is_err());
    let (one, two) = tokio::join!(
        ts.execute(&a, &c.id, cmd.clone()),
        ts.execute(&a, &c.id, cmd.clone())
    );
    let one = one.unwrap();
    assert_eq!(one.revision, two.unwrap().revision);
    assert_eq!(ts.list(&a, &c.id, 1).await.unwrap().total, 1);
    assert_eq!(
        draft::list(&ds, &a, &c.id, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
    assert!(draft::get(&ds, &a, &c.id, &d.id).await.is_err());
    assert!(draft::update(
        &ds,
        &a,
        &c.id,
        &d.id,
        draft::Update {
            revision: one.revision,
            data: d.data.clone()
        }
    )
    .await
    .is_err());
    assert!(draft::issue(
        &ds,
        &a,
        &c.id,
        &d.id,
        draft::Issue {
            request_id: key(),
            revision: one.revision
        }
    )
    .await
    .is_err());
    assert!(sqlx::query(
        "UPDATE invoice_draft SET number='SHOULD-NOT-ISSUE' WHERE id=$1::text::bigint"
    )
    .bind(&d.id)
    .execute(&p)
    .await
    .is_err());
    let restored = ts
        .execute(
            &a,
            &c.id,
            Command {
                request_id: key(),
                id: d.id.clone(),
                revision: one.revision,
                action: Action::Restore,
            },
        )
        .await
        .unwrap();
    assert!(restored.deleted_at.is_none());
    assert_eq!(ts.list(&a, &c.id, 1).await.unwrap().total, 0);
    assert_eq!(
        draft::get(&ds, &a, &c.id, &d.id).await.unwrap().data.note,
        d.data.note
    );
    let v = faktura_server::draft_trash::tool(
        p.clone(),
        &a,
        "invoice_trash_list",
        serde_json::json!({"company_id":c.id})
            .as_object()
            .unwrap()
            .clone(),
    )
    .await
    .unwrap();
    assert_eq!(v["total"], 0);
    let issued = draft::issue(
        &ds,
        &a,
        &c.id,
        &d.id,
        draft::Issue {
            request_id: key(),
            revision: restored.revision,
        },
    )
    .await
    .unwrap();
    assert!(ts
        .execute(
            &a,
            &c.id,
            Command {
                request_id: key(),
                id: d.id.clone(),
                revision: issued.revision,
                action: Action::Trash
            }
        )
        .await
        .is_err());
    assert!(sqlx::query(
        "UPDATE invoice_trash SET deleted_at=now() WHERE draft_id=$1::text::bigint"
    )
    .bind(&d.id)
    .execute(&p)
    .await
    .is_err());
    // A race between issuing and trashing a fresh draft has exactly one winner.
    let fresh = draft::create(
        &ds,
        &a,
        &c.id,
        draft::Create {
            request_id: key(),
            data: d.data,
        },
    )
    .await
    .unwrap();
    let (issued, trashed) = tokio::join!(
        draft::issue(
            &ds,
            &a,
            &c.id,
            &fresh.id,
            draft::Issue {
                request_id: key(),
                revision: fresh.revision
            }
        ),
        ts.execute(
            &a,
            &c.id,
            Command {
                request_id: key(),
                id: fresh.id.clone(),
                revision: fresh.revision,
                action: Action::Trash
            }
        )
    );
    assert_ne!(issued.is_ok(), trashed.is_ok());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn export_lock_blocks_trash(p: PgPool) {
    use faktura_application::document_exports::{ExportStore, Kind, Request, Version};
    use faktura_infrastructure::document_exports::PgDocumentExports;
    let a:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('trash-locked@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let a = a.to_string();
    let c = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let d = draft::create(
        &PgDraft(p.clone()),
        &a,
        &c.id,
        draft::Create {
            request_id: key(),
            data: serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap(),
        },
    )
    .await
    .unwrap();
    PgDocumentExports(p.clone())
        .create(
            &a,
            &c.id,
            Request {
                request_id: key(),
                targets: vec![Version {
                    kind: Kind::Invoice,
                    id: d.id.clone(),
                    revision: d.revision,
                }],
                lock: true,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        PgTrash(p)
            .execute(
                &a,
                &c.id,
                Command {
                    request_id: key(),
                    id: d.id,
                    revision: d.revision,
                    action: Action::Trash
                }
            )
            .await,
        Err(CompanyError::Conflict)
    ));
}

use faktura_application::{
    company,
    company::CompanyError,
    document_exports::*,
    draft::{self},
};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{
    company::PgCompany, document_exports::PgDocumentExports, draft::PgDraft,
};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn export_replay_revision_lock_owner_unlock_and_isolation(p: PgPool) {
    let a:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('export-lock@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let a = a.to_string();
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('export-viewer@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let c = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let other = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
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
    let drafts = PgDraft(p.clone());
    let data =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    let draft = draft::create(
        &drafts,
        &a,
        &c.id,
        draft::Create {
            request_id: key(),
            data,
        },
    )
    .await
    .unwrap();
    let target = Target {
        kind: Kind::Invoice,
        id: draft.id.clone(),
    };
    let store = PgDocumentExports(p.clone());
    let request = Request {
        request_id: key(),
        targets: vec![Version {
            kind: Kind::Invoice,
            id: draft.id.clone(),
            revision: draft.revision,
        }],
        lock: true,
    };
    assert!(matches!(
        store
            .create(&viewer.to_string(), &c.id, request.clone())
            .await,
        Err(CompanyError::Forbidden)
    ));
    let file = store.create(&a, &c.id, request.clone()).await.unwrap();
    assert_eq!(
        file.bytes,
        store.create(&a, &c.id, request).await.unwrap().bytes
    );
    let content: serde_json::Value = serde_json::from_slice(&file.bytes).unwrap();
    assert_eq!(content["documents"][0]["revision"], 1);
    let history = store.history(&a, &c.id, target.clone()).await.unwrap();
    assert_eq!(history.exports.len(), 1);
    let export = history.locked_by_export.unwrap();
    assert_eq!(
        store.file(&a, &c.id, &export).await.unwrap().bytes,
        file.bytes
    );
    assert!(store.file(&a, &other.id, &export).await.is_err());
    let mut data = draft.data.clone();
    data.note = "Blocked edit".into();
    assert!(matches!(
        draft::update(
            &drafts,
            &a,
            &c.id,
            &draft.id,
            draft::Update {
                revision: draft.revision,
                data
            }
        )
        .await,
        Err(CompanyError::Conflict)
    ));
    let unlock = Unlock {
        request_id: key(),
        target: target.clone(),
        export_id: export.clone(),
        reason: "Correct original input".into(),
    };
    assert!(matches!(
        store
            .unlock(&viewer.to_string(), &c.id, unlock.clone())
            .await,
        Err(CompanyError::Forbidden)
    ));
    assert!(store
        .unlock(&a, &c.id, unlock.clone())
        .await
        .unwrap()
        .locked_by_export
        .is_none());
    assert!(store
        .unlock(&a, &c.id, unlock)
        .await
        .unwrap()
        .locked_by_export
        .is_none());
    let mut data = draft.data.clone();
    data.note = "Allowed corrected input".into();
    let updated = draft::update(
        &drafts,
        &a,
        &c.id,
        &draft.id,
        draft::Update {
            revision: draft.revision,
            data,
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(
        store.file(&a, &c.id, &export).await.unwrap().bytes,
        file.bytes
    );
    assert!(matches!(
        store
            .create(
                &a,
                &c.id,
                Request {
                    request_id: key(),
                    targets: vec![Version {
                        kind: Kind::Invoice,
                        id: draft.id,
                        revision: 1
                    }],
                    lock: true
                }
            )
            .await,
        Err(CompanyError::Conflict)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn content_guards_allow_received_payments_sales_advance_and_pdf_cache(p: PgPool) {
    let a:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('guards@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = a.to_string();
    let c = company::create(
        &PgCompany(p.clone()),
        &actor,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let cid = c.id.parse::<i64>().unwrap();
    let sales:i64=sqlx::query_scalar("INSERT INTO sales_document(company_id,creator_id,kind,request_id,initial_data,data) VALUES($1,$2,'advance',$3::text::uuid,'{}','{}') RETURNING id").bind(cid).bind(a).bind(key()).fetch_one(&p).await.unwrap();
    let received:i64=sqlx::query_scalar("INSERT INTO received_invoice(company_id,number,data,total,received_on,due_on) VALUES($1,'SYN-1','{}',100,current_date,current_date) RETURNING id").bind(cid).fetch_one(&p).await.unwrap();
    let store = PgDocumentExports(p.clone());
    store
        .create(
            &actor,
            &c.id,
            Request {
                request_id: key(),
                targets: vec![
                    Version {
                        kind: Kind::Sales,
                        id: sales.to_string(),
                        revision: 1,
                    },
                    Version {
                        kind: Kind::Received,
                        id: received.to_string(),
                        revision: 1,
                    },
                ],
                lock: true,
            },
        )
        .await
        .unwrap();
    sqlx::query("UPDATE received_invoice SET paid=1,payments='[{\"synthetic\":true}]',revision=revision+1 WHERE company_id=$1 AND id=$2").bind(cid).bind(received).execute(&p).await.unwrap();
    sqlx::query("UPDATE sales_document SET advance_paid=1,revision=revision+1 WHERE company_id=$1 AND id=$2").bind(cid).bind(sales).execute(&p).await.unwrap();
    for (table, document) in [("received_invoice", received), ("sales_document", sales)] {
        let error = sqlx::query(&format!(
            "UPDATE {table} SET data='{{\"changed\":true}}' WHERE company_id=$1 AND id=$2"
        ))
        .bind(cid)
        .bind(document)
        .execute(&p)
        .await
        .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().constraint(),
            Some("document_export_locked")
        );
    }
    let events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM document_export_event WHERE company_id=$1")
            .bind(cid)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(events, 1);
}

use faktura_application::{company, company::CompanyError, document_tags::*};
use faktura_domain::company::CompanySettings;
use faktura_infrastructure::{company::PgCompany, document_tags::PgTags};
use sqlx::PgPool;
fn command(action: Action) -> Command {
    Command {
        request_id: uuid::Uuid::new_v4().to_string(),
        action,
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tags_revision_replay_scope_roles_and_foreign_keys(p: PgPool) {
    let actor:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('tags@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let viewer:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('tags-viewer@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = actor.to_string();
    let viewer = viewer.to_string();
    let a = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let b = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1,$2,'viewer')")
        .bind(a.id.parse::<i64>().unwrap())
        .bind(viewer.parse::<i64>().unwrap())
        .execute(&p)
        .await
        .unwrap();
    let store = PgTags(p.clone());
    let create = command(Action::Create {
        name: "Project".into(),
    });
    let one = store.execute(&actor, &a.id, create.clone()).await.unwrap();
    assert_eq!(one.len(), 1);
    let tag = one[0].id.clone();
    assert_eq!(
        store.execute(&actor, &a.id, create).await.unwrap()[0].id,
        tag
    );
    assert!(matches!(
        store
            .execute(
                &viewer,
                &a.id,
                command(Action::Create {
                    name: "Blocked".into()
                })
            )
            .await,
        Err(CompanyError::Forbidden)
    ));
    assert_eq!(store.list(&viewer, &a.id, None).await.unwrap().len(), 1);
    assert!(store.list(&viewer, &b.id, None).await.is_err());
    assert!(store.list(&actor, &b.id, None).await.unwrap().is_empty());
    assert!(matches!(
        store
            .execute(
                &actor,
                &a.id,
                command(Action::Create {
                    name: "project".into()
                })
            )
            .await,
        Err(CompanyError::Conflict)
    ));
    let cid:i64=sqlx::query_scalar("INSERT INTO contact(company_id,creator_id,request_id,initial_data,data) VALUES($1,$2,$3::text::uuid,'{}','{}') RETURNING id").bind(a.id.parse::<i64>().unwrap()).bind(actor.parse::<i64>().unwrap()).bind(uuid::Uuid::new_v4().to_string()).fetch_one(&p).await.unwrap();
    let target = Target {
        kind: Kind::Contact,
        id: cid.to_string(),
    };
    let attach = command(Action::Attach {
        tag_id: tag.clone(),
        target: target.clone(),
    });
    store.execute(&actor, &a.id, attach.clone()).await.unwrap();
    store.execute(&actor, &a.id, attach).await.unwrap();
    assert_eq!(
        store
            .list(&actor, &a.id, Some(target.clone()))
            .await
            .unwrap()[0]
            .usage_count,
        1
    );
    assert!(matches!(
        store
            .execute(
                &actor,
                &b.id,
                command(Action::Attach {
                    tag_id: tag.clone(),
                    target: target.clone()
                })
            )
            .await,
        Err(CompanyError::NotFound)
    ));
    let renamed = store
        .execute(
            &actor,
            &a.id,
            command(Action::Rename {
                tag_id: tag.clone(),
                revision: 1,
                name: "Renamed".into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(renamed[0].revision, 2);
    assert!(matches!(
        store
            .execute(
                &actor,
                &a.id,
                command(Action::Rename {
                    tag_id: tag.clone(),
                    revision: 1,
                    name: "Stale".into()
                })
            )
            .await,
        Err(CompanyError::Conflict)
    ));
    store
        .execute(
            &actor,
            &a.id,
            command(Action::Delete {
                tag_id: tag,
                revision: 2,
            }),
        )
        .await
        .unwrap();
    assert!(store
        .list(&actor, &a.id, Some(target))
        .await
        .unwrap()
        .is_empty());
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contact WHERE id=$1)")
        .bind(cid)
        .fetch_one(&p)
        .await
        .unwrap();
    assert!(exists);
}
#[test]
fn command_json_roundtrip_and_unknown_action() {
    let value = serde_json::json!({"request_id":uuid::Uuid::new_v4().to_string(),"action":"attach","tag_id":"1","target":{"kind":"invoice","id":"1"}});
    let cmd: Command = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(cmd).unwrap(), value);
    assert!(serde_json::from_value::<Command>(
        serde_json::json!({"request_id":"x","action":"drop_all"})
    )
    .is_err());
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn tag_filters_before_pagination_all_registers(p: PgPool) {
    use faktura_application::{
        contact::{self, ContactStore},
        draft::{self, DraftStore},
        received::{self, ReceivedStore},
        sales::{self, SalesStore},
    };
    use faktura_infrastructure::{
        contact::PgContact, draft::PgDraft, received::PgReceived, sales::PgSales,
    };
    let actor:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('tag-filters@example.test','synthetic') RETURNING id").fetch_one(&p).await.unwrap();
    let actor = actor.to_string();
    let c = company::create(
        &PgCompany(p.clone()),
        &actor,
        &uuid::Uuid::new_v4().to_string(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let store = PgTags(p.clone());
    let tag = store
        .execute(
            &actor,
            &c.id,
            command(Action::Create {
                name: "Filter project".into(),
            }),
        )
        .await
        .unwrap()[0]
        .id
        .clone();
    let mut contacts = Vec::new();
    let mut invoices = Vec::new();
    let mut quotes = Vec::new();
    let mut purchases = Vec::new();
    for n in 0..3 {
        let contact = contact::create(
            &PgContact(p.clone()),
            &actor,
            &c.id,
            contact::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data: faktura_domain::contact::ContactData {
                    name: format!("Customer {n}"),
                    billing_address: faktura_domain::contact::Address {
                        country: "CZ".into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            },
        )
        .await
        .unwrap();
        contacts.push(contact.id);
        let mut data: faktura_domain::invoice::DraftData =
            serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json"))
                .unwrap();
        data.supplier.registration_number = "12345678".into();
        data.payment.method = "cash".into();
        data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
        data.calculation.lines[0].vat_percent = "0".into();
        data.tax_on = None;
        let invoice = draft::create(
            &PgDraft(p.clone()),
            &actor,
            &c.id,
            draft::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data: data.clone(),
            },
        )
        .await
        .unwrap();
        invoices.push(invoice.id);
        let quote = sales::create(
            &PgSales(p.clone()),
            &actor,
            &c.id,
            sales::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                kind: sales::Kind::Quote,
                data: sales::Data {
                    name: format!("Quote {n}"),
                    maturity_days: 14,
                    status: "created".into(),
                    invoice: data.clone(),
                },
            },
        )
        .await
        .unwrap();
        quotes.push(quote.id);
        let received:faktura_domain::received::Data=serde_json::from_value(serde_json::json!({"supplier_contact_id":null,"supplier":data.supplier,"external_number":format!("PURCHASE-{n}"),"variable_symbol":"123","bank_account":"","issued_on":"2026-10-01","received_on":"2026-10-18","due_on":"2026-10-31","tax_on":null,"currency":"CZK","description":"Synthetic purchase","note":"","lines":[{"description":"Service","rate":"0","net":"100","vat":"0"}],"rounding":"0"})).unwrap();
        let purchase = received::create(
            &PgReceived(p.clone()),
            &actor,
            &c.id,
            received::Create {
                request_id: uuid::Uuid::new_v4().to_string(),
                data: received,
            },
        )
        .await
        .unwrap();
        purchases.push(purchase.id);
    }
    for (kind, ids) in [
        (Kind::Contact, &contacts),
        (Kind::Invoice, &invoices),
        (Kind::Sales, &quotes),
        (Kind::Received, &purchases),
    ] {
        for index in [0, 2] {
            store
                .execute(
                    &actor,
                    &c.id,
                    command(Action::Attach {
                        tag_id: tag.clone(),
                        target: Target {
                            kind: kind.clone(),
                            id: ids[index].clone(),
                        },
                    }),
                )
                .await
                .unwrap();
        }
    }
    let first = PgContact(p.clone())
        .list(
            &actor,
            &c.id,
            contact::Search {
                tag_id: tag.clone(),
                page_size: 1,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let second = PgContact(p.clone())
        .list(
            &actor,
            &c.id,
            contact::Search {
                tag_id: tag.clone(),
                page_size: 1,
                page: 2,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(second.total, 2);
    assert_ne!(first.items[0].id, second.items[0].id);
    let invoice = PgDraft(p.clone())
        .list(
            &actor,
            &c.id,
            draft::Search {
                tag_id: tag.clone(),
                page_size: 1,
                page: 2,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(invoice.total, 2);
    assert_eq!(invoice.items.len(), 1);
    let sales = PgSales(p.clone())
        .list(
            &actor,
            &c.id,
            sales::Search {
                tag_id: tag.clone(),
                page_size: 1,
                page: 2,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(sales.total, 2);
    assert_eq!(sales.items.len(), 1);
    let received = PgReceived(p.clone())
        .list(
            &actor,
            &c.id,
            received::Search {
                tag_id: tag.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(received.total, 2);
    assert_eq!(received.items.len(), 2);
    assert_eq!(
        PgDraft(p.clone())
            .list(
                &actor,
                &c.id,
                draft::Search {
                    tag_id: "999999999999".into(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .total,
        0
    );
}

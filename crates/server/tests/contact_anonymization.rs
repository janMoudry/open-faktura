use faktura_application::{
    company,
    company::CompanyError as Error,
    contact::{self, ContactStore},
    contact_anonymization::{AnonymizationStore, Command},
    draft::{self, DraftStore},
};
use faktura_domain::{
    company::CompanySettings,
    contact::{Address, ContactData, DeliveryAddress, Person},
};
use faktura_infrastructure::{
    company::PgCompany, contact::PgContact, contact_anonymization::PgContactAnonymization,
    draft::PgDraft,
};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn data() -> ContactData {
    ContactData {
        name: "Synthetic confidential subject".into(),
        email: "private-subject@example.test".into(),
        phone: "+420123456789".into(),
        note: "private synthetic note".into(),
        bank_account: "123456789/0800".into(),
        persons: vec![Person {
            name: "Synthetic person".into(),
            email: "person@example.test".into(),
            phone: "private person phone".into(),
        }],
        recipients: vec!["recipient@example.test".into()],
        billing_address: Address {
            street: "Private synthetic street".into(),
            country: "CZ".into(),
            ..Default::default()
        },
        delivery_addresses: vec![DeliveryAddress {
            id: "delivery".into(),
            name: "Private warehouse".into(),
            address: Address {
                country: "CZ".into(),
                ..Default::default()
            },
        }],
        ..Default::default()
    }
}
fn command(revision: i64) -> Command {
    Command {
        request_id: key(),
        revision,
        confirm: true,
    }
}
async fn setup(p: &PgPool) -> (String, String, String) {
    let a:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('anon-owner@example.test','synthetic') RETURNING id").fetch_one(p).await.unwrap();
    let b:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES('anon-member@example.test','synthetic',true) RETURNING id").fetch_one(p).await.unwrap();
    let a = a.to_string();
    let b = b.to_string();
    let c = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    (a, b, c.id)
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn contact_anonymization_owner_confirmation_replay_and_terminal_history(p: PgPool) {
    let (a, b, c) = setup(&p).await;
    let contacts = PgContact(p.clone());
    let anon = PgContactAnonymization(p.clone());
    let request = key();
    let original = data();
    let first = contact::create(
        &contacts,
        &a,
        &c,
        contact::Create {
            request_id: request.clone(),
            data: original.clone(),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        anon.execute(&b, &c, &first.id, command(1)).await,
        Err(Error::NotFound)
    ));
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1::text::bigint,$2::text::bigint,'viewer')").bind(&c).bind(&b).execute(&p).await.unwrap();
    for role in ["viewer", "editor"] {
        sqlx::query("UPDATE membership SET role=$3 WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&b).bind(role).execute(&p).await.unwrap();
        assert!(matches!(
            anon.execute(&b, &c, &first.id, command(1)).await,
            Err(Error::Forbidden)
        ));
    }
    let mut invalid = command(1);
    invalid.confirm = false;
    assert!(matches!(
        anon.execute(&a, &c, &first.id, invalid).await,
        Err(Error::Invalid(_))
    ));
    let edited = contact::update(
        &contacts,
        &a,
        &c,
        &first.id,
        contact::Update {
            revision: 1,
            archived: false,
            data: ContactData {
                note: "latest private synthetic note".into(),
                ..original.clone()
            },
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        anon.execute(&a, &c, &first.id, command(1)).await,
        Err(Error::Conflict)
    ));
    let cmd = command(edited.revision);
    let result = anon.execute(&a, &c, &first.id, cmd.clone()).await.unwrap();
    assert!(result.archived && result.anonymized);
    assert_eq!(result.revision, 3);
    assert_eq!(
        anon.execute(&a, &c, &first.id, cmd.clone())
            .await
            .unwrap()
            .revision,
        3
    );
    let mut changed = cmd.clone();
    changed.revision = 3;
    assert!(matches!(
        anon.execute(&a, &c, &first.id, changed).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        contact::update(
            &contacts,
            &a,
            &c,
            &first.id,
            contact::Update {
                revision: 3,
                archived: false,
                data: original.clone()
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        contact::create(
            &contacts,
            &a,
            &c,
            contact::Create {
                request_id: request,
                data: original
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    let current = contacts.get(&a, &c, &first.id).await.unwrap();
    assert!(current.anonymized);
    assert!(current.data.email.is_empty());
    assert!(current.data.persons.is_empty());
    let snapshots:Vec<serde_json::Value>=sqlx::query_scalar("SELECT data FROM contact_audit WHERE company_id=$1::text::bigint AND contact_id=$2::text::bigint UNION ALL SELECT initial_data FROM contact WHERE company_id=$1::text::bigint AND id=$2::text::bigint UNION ALL SELECT result FROM contact_anonymization_command WHERE company_id=$1::text::bigint AND contact_id=$2::text::bigint").bind(&c).bind(&first.id).fetch_all(&p).await.unwrap();
    for snapshot in snapshots {
        let text = snapshot.to_string();
        assert!(!text.contains("private"));
        assert!(!text.contains("Private"));
        assert!(!text.contains("Synthetic confidential"));
        assert!(!text.contains("person@example"));
        assert!(!text.contains("123456789"));
    }
    assert!(
        sqlx::query("UPDATE contact SET data='{}',archived=false WHERE id=$1::text::bigint")
            .bind(&first.id)
            .execute(&p)
            .await
            .is_err()
    );
    sqlx::query("UPDATE membership SET role='editor' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&a).execute(&p).await.unwrap();
    assert!(matches!(
        anon.execute(&a, &c, &first.id, cmd).await,
        Err(Error::Forbidden)
    ));
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn contact_anonymization_concurrency_and_company_isolation(p: PgPool) {
    let (a, _, c) = setup(&p).await;
    let s = PgContactAnonymization(p.clone());
    let first = contact::create(
        &PgContact(p.clone()),
        &a,
        &c,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let same = command(1);
    let (one, two) = tokio::join!(
        s.execute(&a, &c, &first.id, same.clone()),
        s.execute(&a, &c, &first.id, same)
    );
    assert_eq!(one.unwrap().revision, two.unwrap().revision);
    let second = contact::create(
        &PgContact(p.clone()),
        &a,
        &c,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let (one, two) = tokio::join!(
        s.execute(&a, &c, &second.id, command(1)),
        s.execute(&a, &c, &second.id, command(1))
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let other = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    assert!(matches!(
        s.execute(&a, &other.id, &first.id, command(2)).await,
        Err(Error::NotFound)
    ));
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM contact_audit WHERE company_id=$1::text::bigint AND action='anonymize'").bind(&c).fetch_one(&p).await.unwrap();
    assert_eq!(count, 2);
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn contact_anonymization_preserves_issued_snapshot_and_original_pdf(p: PgPool) {
    let (a, _, c) = setup(&p).await;
    let contact = contact::create(
        &PgContact(p.clone()),
        &a,
        &c,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let mut data: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    let today: sqlx::types::chrono::NaiveDate = sqlx::query_scalar("SELECT current_date")
        .fetch_one(&p)
        .await
        .unwrap();
    data.issued_on = Some(today);
    data.due_on = Some(today);
    data.tax_on = None;
    data.payment.method = "cash".into();
    data.supplier.registration_number = "12345678".into();
    data.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    data.calculation.lines[0].vat_percent = "0".into();
    data.contact_id = Some(contact.id.clone());
    data.customer.as_mut().unwrap().name = contact.data.name.clone();
    data.customer.as_mut().unwrap().email = contact.data.email.clone();
    let s = PgDraft(p.clone());
    let draft = draft::create(
        &s,
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data,
        },
    )
    .await
    .unwrap();
    let invoice = draft::issue(
        &s,
        &a,
        &c,
        &draft.id,
        draft::Issue {
            request_id: key(),
            revision: draft.revision,
        },
    )
    .await
    .unwrap();
    let original = s.pdf(&a, &c, &invoice.id).await.unwrap();
    let raw: serde_json::Value =
        sqlx::query_scalar("SELECT data FROM invoice_draft WHERE id=$1::text::bigint")
            .bind(&invoice.id)
            .fetch_one(&p)
            .await
            .unwrap();
    PgContactAnonymization(p.clone())
        .execute(&a, &c, &contact.id, command(1))
        .await
        .unwrap();
    assert_eq!(s.pdf(&a, &c, &invoice.id).await.unwrap(), original);
    let after: serde_json::Value =
        sqlx::query_scalar("SELECT data FROM invoice_draft WHERE id=$1::text::bigint")
            .bind(&invoice.id)
            .fetch_one(&p)
            .await
            .unwrap();
    assert_eq!(raw, after);
    assert_eq!(after["customer"]["name"], "Synthetic confidential subject");
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn concurrent_contact_update_and_anonymization_do_not_restore_private_history(p: PgPool) {
    let (a, _, c) = setup(&p).await;
    let contacts = PgContact(p.clone());
    let anon = PgContactAnonymization(p.clone());
    let first = contact::create(
        &contacts,
        &a,
        &c,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(
            anon.execute(&a, &c, &first.id, command(1)),
            contact::update(
                &contacts,
                &a,
                &c,
                &first.id,
                contact::Update {
                    revision: 1,
                    archived: false,
                    data: ContactData {
                        note: "racing private note".into(),
                        ..data()
                    }
                }
            )
        )
    })
    .await
    .expect("consistent company/contact lock order must not deadlock");
    assert_eq!(
        usize::from(outcome.0.is_ok()) + usize::from(outcome.1.is_ok()),
        1
    );
    let current = contacts.get(&a, &c, &first.id).await.unwrap();
    if !current.anonymized {
        anon.execute(&a, &c, &first.id, command(current.revision))
            .await
            .unwrap();
    }
    let forbidden:i64=sqlx::query_scalar("SELECT count(*) FROM contact_audit WHERE company_id=$1::text::bigint AND data::text LIKE '%private%'").bind(&c).fetch_one(&p).await.unwrap();
    assert_eq!(forbidden, 0);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn contact_anonymization_rest_and_mcp_share_commands(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use faktura_application::auth;
    use faktura_infrastructure::auth::PgAuth;
    use tower::ServiceExt;
    let auth = PgAuth::new(p.clone(), Some("synthetic-anon-bootstrap".into()));
    auth::bootstrap(
        &auth,
        "synthetic-anon-bootstrap",
        "anon-transport@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (actor, token) = auth::login(
        &auth,
        "anon-transport@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let company = company::create(
        &PgCompany(p.clone()),
        &actor.id,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let contact = contact::create(
        &PgContact(p.clone()),
        &actor.id,
        &company.id,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let app = faktura_server::contact_anonymization::router(
        p.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    let command = command(1);
    let body = serde_json::to_value(&command).unwrap();
    for (session, origin, status) in [
        ("", "https://app.example.test", StatusCode::UNAUTHORIZED),
        (
            token.as_str(),
            "https://evil.example.test",
            StatusCode::UNAUTHORIZED,
        ),
        (token.as_str(), "https://app.example.test", StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/companies/{}/contacts/{}/anonymize",
                        company.id, contact.id
                    ))
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .header("X-Faktura-Request", "1")
                    .header("Origin", origin)
                    .header("Cookie", format!("faktura_session={session}"))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if status == StatusCode::OK {
            let response: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap())
                    .unwrap();
            assert_eq!(response["anonymized"], true);
        }
    }
    let mut args = body.as_object().unwrap().clone();
    args.insert("company_id".into(), company.id.clone().into());
    args.insert("contact_id".into(), contact.id.clone().into());
    let replay = faktura_server::contact_anonymization::tool(
        p.clone(),
        &actor.id,
        "contact_anonymize",
        args.clone(),
    )
    .await
    .unwrap();
    assert_eq!(replay["revision"], 2);
    assert_eq!(replay["anonymized"], true);
    args.insert("unexpected".into(), true.into());
    assert!(matches!(
        faktura_server::contact_anonymization::tool(p, &actor.id, "contact_anonymize", args).await,
        Err(Error::Invalid(_))
    ));
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn anonymized_contact_rejects_new_links_but_preserves_existing_owned_snapshots(p: PgPool) {
    use faktura_application::{
        recurring::{self},
        sales::{self, Kind},
    };
    use faktura_infrastructure::{recurring::PgRecurring, sales::PgSales};
    let (a, _, c) = setup(&p).await;
    let contacts = PgContact(p.clone());
    let drafts = PgDraft(p.clone());
    let sales = PgSales(p.clone());
    let recurring = PgRecurring(p.clone());
    let contact = contact::create(
        &contacts,
        &a,
        &c,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    let mut invoice: faktura_domain::invoice::DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.tax_on = None;
    invoice.calculation.tax_mode = faktura_domain::invoice::TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    invoice.contact_id = Some(contact.id.clone());
    invoice.customer.as_mut().unwrap().name = contact.data.name.clone();
    let request = key();
    let existing = draft::create(
        &drafts,
        &a,
        &c,
        draft::Create {
            request_id: request.clone(),
            data: invoice.clone(),
        },
    )
    .await
    .unwrap();
    let mut no_link = invoice.clone();
    no_link.contact_id = None;
    let unbound = draft::create(
        &drafts,
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data: no_link.clone(),
        },
    )
    .await
    .unwrap();
    let sales_data = sales::Data {
        name: "Existing synthetic source".into(),
        maturity_days: 14,
        status: "created".into(),
        invoice: invoice.clone(),
    };
    let bound_template = sales::create(
        &sales,
        &a,
        &c,
        sales::Create {
            request_id: key(),
            kind: Kind::Template,
            data: sales_data.clone(),
        },
    )
    .await
    .unwrap();
    let unbound_template = sales::create(
        &sales,
        &a,
        &c,
        sales::Create {
            request_id: key(),
            kind: Kind::Template,
            data: sales::Data {
                invoice: no_link.clone(),
                ..sales_data.clone()
            },
        },
    )
    .await
    .unwrap();
    let rule_data = recurring::Data {
        rate_policy: recurring::RatePolicy::Stored,
        name: "Existing synthetic rule".into(),
        schedule: faktura_domain::recurring::Schedule {
            first_on: "2026-10-18".parse().unwrap(),
            frequency: "month".into(),
            interval: 1,
            month_end: false,
            end_on: None,
            count: Some(1),
            timezone: "Europe/Prague".into(),
        },
        maturity_days: 14,
        auto_issue: false,
        mail: None,
        invoice: invoice.clone(),
    };
    let bound_rule = recurring::create(
        &recurring,
        &a,
        &c,
        recurring::Create {
            request_id: key(),
            data: rule_data.clone(),
        },
    )
    .await
    .unwrap();
    let unbound_rule = recurring::create(
        &recurring,
        &a,
        &c,
        recurring::Create {
            request_id: key(),
            data: recurring::Data {
                invoice: no_link.clone(),
                ..rule_data.clone()
            },
        },
    )
    .await
    .unwrap();
    PgContactAnonymization(p.clone())
        .execute(&a, &c, &contact.id, command(1))
        .await
        .unwrap();
    assert!(matches!(
        draft::create(
            &drafts,
            &a,
            &c,
            draft::Create {
                request_id: key(),
                data: invoice.clone()
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    // Historical UUID retry returns its existing document, never creates a new link.
    assert_eq!(
        draft::create(
            &drafts,
            &a,
            &c,
            draft::Create {
                request_id: request,
                data: invoice.clone()
            }
        )
        .await
        .unwrap()
        .id,
        existing.id
    );
    let edited = draft::update(
        &drafts,
        &a,
        &c,
        &existing.id,
        draft::Update {
            revision: 1,
            data: invoice.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        edited.data.customer.unwrap().name,
        "Synthetic confidential subject"
    );
    assert!(matches!(
        draft::update(
            &drafts,
            &a,
            &c,
            &unbound.id,
            draft::Update {
                revision: 1,
                data: invoice.clone()
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    for kind in [Kind::Quote, Kind::Template, Kind::Advance] {
        assert!(matches!(
            sales::create(
                &sales,
                &a,
                &c,
                sales::Create {
                    request_id: key(),
                    kind,
                    data: sales_data.clone()
                }
            )
            .await,
            Err(Error::NotFound)
        ));
    }
    assert!(sales::update(
        &sales,
        &a,
        &c,
        &bound_template.id,
        sales::Update {
            revision: 1,
            archived: false,
            data: sales_data.clone()
        }
    )
    .await
    .is_ok());
    assert!(matches!(
        sales::update(
            &sales,
            &a,
            &c,
            &unbound_template.id,
            sales::Update {
                revision: 1,
                archived: false,
                data: sales_data.clone()
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        sales::convert(
            &sales,
            &a,
            &c,
            &bound_template.id,
            sales::Convert {
                request_id: key(),
                revision: 2,
                issued_on: "2026-10-18".parse().unwrap()
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        recurring::create(
            &recurring,
            &a,
            &c,
            recurring::Create {
                request_id: key(),
                data: rule_data.clone()
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(recurring::command(
        &recurring,
        &a,
        &c,
        &bound_rule.id,
        recurring::Command {
            request_id: key(),
            revision: 1,
            action: recurring::Action::Save {
                data: Box::new(rule_data.clone())
            }
        }
    )
    .await
    .is_ok());
    assert!(matches!(
        recurring::command(
            &recurring,
            &a,
            &c,
            &unbound_rule.id,
            recurring::Command {
                request_id: key(),
                revision: 1,
                action: recurring::Action::Save {
                    data: Box::new(rule_data)
                }
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    // Detaching is allowed, but reattaching the retired ID is a new association.
    let detached = draft::update(
        &drafts,
        &a,
        &c,
        &existing.id,
        draft::Update {
            revision: 2,
            data: no_link.clone(),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        draft::update(
            &drafts,
            &a,
            &c,
            &existing.id,
            draft::Update {
                revision: detached.revision,
                data: invoice
            }
        )
        .await,
        Err(Error::NotFound)
    ));
    // Ordinary archival keeps its pre-existing reference semantics.
    let archived = contact::create(
        &contacts,
        &a,
        &c,
        contact::Create {
            request_id: key(),
            data: data(),
        },
    )
    .await
    .unwrap();
    contact::update(
        &contacts,
        &a,
        &c,
        &archived.id,
        contact::Update {
            revision: 1,
            archived: true,
            data: archived.data,
        },
    )
    .await
    .unwrap();
    no_link.contact_id = Some(archived.id);
    assert!(draft::create(
        &drafts,
        &a,
        &c,
        draft::Create {
            request_id: key(),
            data: no_link
        }
    )
    .await
    .is_ok());
}

use faktura_application::{
    company,
    company::CompanyError as Error,
    sales::{self, Kind, SalesStore},
    sales_trash::*,
};
use faktura_domain::{
    company::CompanySettings,
    invoice::{DraftData, TaxMode},
};
use faktura_infrastructure::{company::PgCompany, sales::PgSales, sales_trash::PgSalesTrash};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn create(kind: Kind) -> sales::Create {
    let mut invoice: DraftData =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    invoice.supplier.registration_number = "12345678".into();
    invoice.payment.method = "cash".into();
    invoice.tax_on = None;
    invoice.calculation.tax_mode = TaxMode::NonPayer;
    invoice.calculation.lines[0].vat_percent = "0".into();
    sales::Create {
        request_id: key(),
        kind,
        data: sales::Data {
            name: "Synthetic sales trash".into(),
            maturity_days: 14,
            status: "created".into(),
            invoice,
        },
    }
}
fn command(id: &str, revision: i64, action: Action) -> Command {
    Command {
        request_id: key(),
        id: id.into(),
        revision,
        action,
    }
}
async fn setup(p: &PgPool) -> (String, String, String, String) {
    let a:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash) VALUES('sales-trash@example.test','synthetic') RETURNING id").fetch_one(p).await.unwrap();
    let b:i64=sqlx::query_scalar("INSERT INTO app_user(email,password_hash,installation_admin) VALUES('sales-trash-viewer@example.test','synthetic',true) RETURNING id").fetch_one(p).await.unwrap();
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
    let other = company::create(
        &PgCompany(p.clone()),
        &a,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO membership(company_id,user_id,role) VALUES($1::text::bigint,$2::text::bigint,'viewer')").bind(&c.id).bind(&b).execute(p).await.unwrap();
    (a, b, c.id, other.id)
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn sales_trash_scope_roles_revision_replay_restore_and_audit(p: PgPool) {
    let (a, b, c, other) = setup(&p).await;
    let store = PgSales(p.clone());
    let trash = PgSalesTrash(p.clone());
    for kind in [Kind::Quote, Kind::Template] {
        let input = create(kind.clone());
        let value = sales::create(&store, &a, &c, input.clone()).await.unwrap();
        let cmd = command(&value.id, value.revision, Action::Trash);
        assert!(matches!(
            trash.execute(&b, &c, cmd.clone()).await,
            Err(Error::Forbidden)
        ));
        assert!(matches!(
            trash.execute(&a, &other, cmd.clone()).await,
            Err(Error::NotFound)
        ));
        let result = trash.execute(&a, &c, cmd.clone()).await.unwrap();
        assert!(result.deleted_at.is_some());
        assert_eq!(result.revision, 2);
        assert_eq!(
            trash.execute(&a, &c, cmd.clone()).await.unwrap().revision,
            2
        );
        assert!(matches!(
            store.get(&a, &c, &value.id).await,
            Err(Error::NotFound)
        ));
        assert!(matches!(
            sales::create(&store, &a, &c, input).await,
            Err(Error::NotFound)
        ));
        assert!(matches!(
            sales::update(
                &store,
                &a,
                &c,
                &value.id,
                sales::Update {
                    revision: 2,
                    archived: false,
                    data: value.data.clone()
                }
            )
            .await,
            Err(Error::NotFound)
        ));
        assert!(matches!(
            sales::convert(
                &store,
                &a,
                &c,
                &value.id,
                sales::Convert {
                    request_id: key(),
                    revision: 2,
                    issued_on: "2026-10-18".parse().unwrap()
                }
            )
            .await,
            Err(Error::NotFound)
        ));
        assert!(store
            .list(
                &a,
                &c,
                sales::Search {
                    kind: kind.clone(),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .items
            .iter()
            .all(|d| d.id != value.id));
        assert!(trash
            .list(&b, &c, 1)
            .await
            .unwrap()
            .items
            .iter()
            .any(|d| d.id == value.id));
        assert!(matches!(
            trash
                .execute(&a, &c, command(&value.id, 1, Action::Restore))
                .await,
            Err(Error::Conflict)
        ));
        let restored = trash
            .execute(&a, &c, command(&value.id, 2, Action::Restore))
            .await
            .unwrap();
        assert!(restored.deleted_at.is_none());
        assert_eq!(restored.revision, 3);
        let current = store.get(&a, &c, &value.id).await.unwrap();
        assert_eq!(
            serde_json::to_value(current.data).unwrap(),
            serde_json::to_value(value.data).unwrap()
        );
        let audits:Vec<String>=sqlx::query_scalar("SELECT action FROM sales_audit WHERE company_id=$1::text::bigint AND document_id=$2::text::bigint ORDER BY revision").bind(&c).bind(&value.id).fetch_all(&p).await.unwrap();
        assert_eq!(audits, vec!["create", "trash", "restore"]);
        // Old replay returns its immutable historical result without deleting again.
        trash.execute(&a, &c, cmd.clone()).await.unwrap();
        assert!(store.get(&a, &c, &value.id).await.is_ok());
        sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&a).execute(&p).await.unwrap();
        assert!(matches!(
            trash.execute(&a, &c, cmd).await,
            Err(Error::Forbidden)
        ));
        sqlx::query("UPDATE membership SET role='owner' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c).bind(&a).execute(&p).await.unwrap();
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn sales_trash_protects_advances_conversions_publication_mail_and_export(p: PgPool) {
    use faktura_application::{
        document_exports::{self, ExportStore},
        share::{self, ShareStore},
    };
    use faktura_infrastructure::{document_exports::PgDocumentExports, share::PgShare};
    let (a, _, c, _) = setup(&p).await;
    let store = PgSales(p.clone());
    let trash = PgSalesTrash(p.clone());
    let advance = sales::create(&store, &a, &c, create(Kind::Advance))
        .await
        .unwrap();
    assert!(matches!(
        trash
            .execute(&a, &c, command(&advance.id, 1, Action::Trash))
            .await,
        Err(Error::Conflict)
    ));
    for kind in [Kind::Quote, Kind::Template] {
        let value = sales::create(&store, &a, &c, create(kind)).await.unwrap();
        sales::convert(
            &store,
            &a,
            &c,
            &value.id,
            sales::Convert {
                request_id: key(),
                revision: 1,
                issued_on: "2026-10-18".parse().unwrap(),
            },
        )
        .await
        .unwrap();
        let value = store.get(&a, &c, &value.id).await.unwrap();
        assert!(matches!(
            trash
                .execute(&a, &c, command(&value.id, value.revision, Action::Trash))
                .await,
            Err(Error::Conflict)
        ));
    }
    let published = sales::create(&store, &a, &c, create(Kind::Quote))
        .await
        .unwrap();
    let shares = PgShare {
        pool: p.clone(),
        base_url: Some("https://synthetic.example.test".into()),
    };
    let link = share::create(
        &shares,
        &a,
        &c,
        &published.id,
        share::Create {
            request_id: key(),
            revision: 1,
            expires_days: 1,
        },
    )
    .await
    .unwrap();
    shares
        .revoke(&a, &c, &published.id, &link.id)
        .await
        .unwrap();
    assert!(matches!(
        trash
            .execute(&a, &c, command(&published.id, 1, Action::Trash))
            .await,
        Err(Error::Conflict)
    ));
    let mailed = sales::create(&store, &a, &c, create(Kind::Quote))
        .await
        .unwrap();
    sqlx::query("INSERT INTO quote_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id,status) VALUES($1::text::bigint,$2::text::bigint,$3::text::bigint,$4::text::uuid,'{}',1,'{}',$5,'sender@example.test','','synthetic@example.test','Synthetic','Synthetic','sales-trash-synthetic','sent')").bind(&c).bind(&mailed.id).bind(&a).bind(key()).bind(b"%PDF-synthetic".as_slice()).execute(&p).await.unwrap();
    assert!(matches!(
        trash
            .execute(&a, &c, command(&mailed.id, 1, Action::Trash))
            .await,
        Err(Error::Conflict)
    ));
    let locked = sales::create(&store, &a, &c, create(Kind::Quote))
        .await
        .unwrap();
    let exports = PgDocumentExports(p.clone());
    exports
        .create(
            &a,
            &c,
            document_exports::Request {
                request_id: key(),
                targets: vec![document_exports::Version {
                    kind: document_exports::Kind::Sales,
                    id: locked.id.clone(),
                    revision: 1,
                }],
                lock: true,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        trash
            .execute(&a, &c, command(&locked.id, 1, Action::Trash))
            .await,
        Err(Error::Conflict)
    ));
    let hidden = sales::create(&store, &a, &c, create(Kind::Quote))
        .await
        .unwrap();
    trash
        .execute(&a, &c, command(&hidden.id, 1, Action::Trash))
        .await
        .unwrap();
    assert!(exports
        .create(
            &a,
            &c,
            document_exports::Request {
                request_id: key(),
                targets: vec![document_exports::Version {
                    kind: document_exports::Kind::Sales,
                    id: hidden.id.clone(),
                    revision: 2
                }],
                lock: true
            }
        )
        .await
        .is_err());
    assert!(share::create(
        &shares,
        &a,
        &c,
        &hidden.id,
        share::Create {
            request_id: key(),
            revision: 2,
            expires_days: 1
        }
    )
    .await
    .is_err());
    assert!(sqlx::query("UPDATE sales_document SET data='{}' WHERE company_id=$1::text::bigint AND id=$2::text::bigint").bind(&c).bind(&hidden.id).execute(&p).await.is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn sales_trash_concurrent_retries_conversion_and_export_are_atomic(p: PgPool) {
    use faktura_application::document_exports::{self, ExportStore};
    use faktura_infrastructure::document_exports::PgDocumentExports;
    let (a, _, c, _) = setup(&p).await;
    let store = PgSales(p.clone());
    let trash = PgSalesTrash(p.clone());
    let value = sales::create(&store, &a, &c, create(Kind::Template))
        .await
        .unwrap();
    let cmd = command(&value.id, 1, Action::Trash);
    let (one, two) = tokio::join!(
        trash.execute(&a, &c, cmd.clone()),
        trash.execute(&a, &c, cmd)
    );
    assert_eq!(one.unwrap().revision, two.unwrap().revision);
    let value = sales::create(&store, &a, &c, create(Kind::Quote))
        .await
        .unwrap();
    let (one, two) = tokio::join!(
        trash.execute(&a, &c, command(&value.id, 1, Action::Trash)),
        sales::convert(
            &store,
            &a,
            &c,
            &value.id,
            sales::Convert {
                request_id: key(),
                revision: 1,
                issued_on: "2026-10-18".parse().unwrap()
            }
        )
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let value = sales::create(&store, &a, &c, create(Kind::Quote))
        .await
        .unwrap();
    let exports = PgDocumentExports(p);
    let (one, two) = tokio::join!(
        trash.execute(&a, &c, command(&value.id, 1, Action::Trash)),
        exports.create(
            &a,
            &c,
            document_exports::Request {
                request_id: key(),
                targets: vec![document_exports::Version {
                    kind: document_exports::Kind::Sales,
                    id: value.id.clone(),
                    revision: 1
                }],
                lock: true
            }
        )
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
}

#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn sales_trash_rest_mcp_share_replay_and_current_permissions(p: PgPool) {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use faktura_application::auth;
    use faktura_infrastructure::auth::PgAuth;
    use tower::ServiceExt;
    let auth = PgAuth::new(p.clone(), Some("synthetic-sales-trash-bootstrap".into()));
    auth::bootstrap(
        &auth,
        "synthetic-sales-trash-bootstrap",
        "sales-trash-transport@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, token) = auth::login(
        &auth,
        "sales-trash-transport@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let c = company::create(
        &PgCompany(p.clone()),
        &a.id,
        &key(),
        CompanySettings::default(),
    )
    .await
    .unwrap();
    let quote = sales::create(&PgSales(p.clone()), &a.id, &c.id, create(Kind::Quote))
        .await
        .unwrap();
    let app = faktura_server::sales_trash::router(
        p.clone(),
        vec!["https://app.example.test".into()],
        false,
    );
    let cmd = command(&quote.id, 1, Action::Trash);
    let body = serde_json::to_value(&cmd).unwrap();
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
                    .method("POST")
                    .uri(format!("/api/companies/{}/sales-trash", c.id))
                    .header("Content-Type", "application/json")
                    .header("Origin", origin)
                    .header("X-Faktura-Request", "1")
                    .header("Cookie", format!("faktura_session={session}"))
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        if status == StatusCode::OK {
            let result: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap())
                    .unwrap();
            assert_eq!(result["revision"], 2);
        }
    }
    let mut args = body.as_object().unwrap().clone();
    args.insert("company_id".into(), c.id.clone().into());
    let replay =
        faktura_server::sales_trash::tool(p.clone(), &a.id, "sales_trash_execute", args.clone())
            .await
            .unwrap();
    assert_eq!(replay["revision"], 2);
    let page = faktura_server::sales_trash::tool(
        p.clone(),
        &a.id,
        "sales_trash_list",
        serde_json::json!({"company_id":c.id})
            .as_object()
            .unwrap()
            .clone(),
    )
    .await
    .unwrap();
    assert_eq!(page["total"], 1);
    let extra = serde_json::json!({"company_id":c.id,"page":1,"unexpected":true});
    assert!(matches!(
        faktura_server::sales_trash::tool(
            p.clone(),
            &a.id,
            "sales_trash_list",
            extra.as_object().unwrap().clone()
        )
        .await,
        Err(Error::Invalid(_))
    ));
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint AND user_id=$2::text::bigint").bind(&c.id).bind(&a.id).execute(&p).await.unwrap();
    assert!(matches!(
        faktura_server::sales_trash::tool(p, &a.id, "sales_trash_execute", args).await,
        Err(Error::Forbidden)
    ));
}

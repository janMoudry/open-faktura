use base64::Engine;
use faktura_application::{
    attachment::AttachmentStore,
    auth, company,
    inbox::{self, InboxStore},
    received::ReceivedStore,
};
use faktura_domain::{
    company::CompanySettings,
    invoice::PartySnapshot,
    received::{Data, Line},
};
use faktura_infrastructure::{
    attachment::PgAttachment, auth::PgAuth, company::PgCompany, inbox::PgInbox,
    received::PgReceived,
};
use sqlx::PgPool;
fn key() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn data() -> Data {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/invoice-draft.json")).unwrap();
    Data {
        exchange_rate: None,
        stock_incidental_cost: None,
        kind: Default::default(),
        receipt_method: None,
        supplier_contact_id: None,
        supplier: serde_json::from_value::<PartySnapshot>(fixture["supplier"].clone()).unwrap(),
        external_number: "VENDOR-001".into(),
        variable_symbol: "000123".into(),
        bank_account: "123/0100".into(),
        issued_on: "2026-10-01".parse().unwrap(),
        received_on: "2026-10-13".parse().unwrap(),
        due_on: "2026-10-15".parse().unwrap(),
        tax_on: None,
        currency: "CZK".into(),
        description: "Synthetic expense".into(),
        note: "Private".into(),
        lines: vec![Line {
            stock: None,
            description: "Service".into(),
            rate: "21".into(),
            net: "100.01".into(),
            vat: "21.02".into(),
        }],
        rounding: "-0.03".into(),
    }
}
async fn setup(p: &PgPool) -> (String, String, String, String) {
    let auth = PgAuth::new(
        p.clone(),
        Some("synthetic-bootstrap-received-only-test".into()),
    );
    auth::bootstrap(
        &auth,
        "synthetic-bootstrap-received-only-test",
        "owner@example.test",
        "Synthetic password 123!",
    )
    .await
    .unwrap();
    let (a, t) = auth::login(&auth, "owner@example.test", "Synthetic password 123!")
        .await
        .unwrap();
    let s = PgCompany(p.clone());
    let c = company::create(&s, &a.id, &key(), CompanySettings::default())
        .await
        .unwrap();
    let o = company::create(&s, &a.id, &key(), CompanySettings::default())
        .await
        .unwrap();
    (a.id, t, c.id, o.id)
}
fn upload() -> inbox::Upload {
    inbox::Upload {
        request_id: key(),
        filename: "synthetic.pdf".into(),
        base64: base64::engine::general_purpose::STANDARD.encode(
            b"%PDF-1.4
synthetic test only",
        ),
    }
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn original_dedup_scope_convert_atomic_replay_archive(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgInbox(p.clone());
    let first = s.upload(&a, &c, upload()).await.unwrap();
    let same = s.upload(&a, &c, upload()).await.unwrap();
    assert_eq!(first.id, same.id);
    assert!(s.download(&a, &other, &first.id).await.is_err());
    let (_, bytes) = s.download(&a, &c, &first.id).await.unwrap();
    assert_eq!(
        bytes,
        b"%PDF-1.4
synthetic test only"
    );
    let cmd = inbox::Command {
        request_id: key(),
        id: first.id.clone(),
        revision: 1,
        action: inbox::Action::Convert {
            data: Box::new(data()),
        },
    };
    let json = serde_json::to_value(&cmd).unwrap();
    let done = s.execute(&a, &c, cmd).await.unwrap();
    let target = done.received_id.clone().unwrap();
    assert_eq!(
        s.execute(&a, &c, serde_json::from_value(json).unwrap())
            .await
            .unwrap()
            .received_id,
        Some(target.clone())
    );
    let attachments = PgAttachment(p.clone())
        .list(&a, &c, "received", &target)
        .await
        .unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].sha256, first.sha256);
    let invoice = PgReceived(p.clone()).get(&a, &c, &target).await.unwrap();
    assert_eq!(invoice.totals.total, "121.00");
    assert_eq!(invoice.paid, "0.00");
    assert!(s
        .execute(
            &a,
            &c,
            inbox::Command {
                request_id: key(),
                id: first.id.clone(),
                revision: done.revision,
                action: inbox::Action::Convert {
                    data: Box::new(data())
                }
            }
        )
        .await
        .is_err());
    let archived = s
        .execute(
            &a,
            &c,
            inbox::Command {
                request_id: key(),
                id: first.id.clone(),
                revision: done.revision,
                action: inbox::Action::Archive { archived: true },
            },
        )
        .await
        .unwrap();
    assert!(archived.archived);
    assert_eq!(s.download(&a, &c, &first.id).await.unwrap().1, bytes);
    assert_eq!(
        PgAttachment(p.clone())
            .list(&a, &c, "received", &target)
            .await
            .unwrap()
            .len(),
        1
    );
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn validation_roles_attachment_limit_and_concurrent_conversion(p: PgPool) {
    let (a, _, c, _) = setup(&p).await;
    let s = PgInbox(p.clone());
    let mut bad = upload();
    bad.filename = "../test.pdf".into();
    assert!(s.upload(&a, &c, bad).await.is_err());
    let mut bad = upload();
    bad.filename = "test.svg".into();
    assert!(s.upload(&a, &c, bad).await.is_err());
    let doc = s.upload(&a, &c, upload()).await.unwrap();
    let mut invalid = data();
    invalid.lines.clear();
    assert!(s
        .execute(
            &a,
            &c,
            inbox::Command {
                request_id: key(),
                id: doc.id.clone(),
                revision: 1,
                action: inbox::Action::Convert {
                    data: Box::new(invalid)
                }
            }
        )
        .await
        .is_err());
    assert_eq!(
        PgReceived(p.clone())
            .list(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        0
    );
    let command = || inbox::Command {
        request_id: key(),
        id: doc.id.clone(),
        revision: 1,
        action: inbox::Action::Convert {
            data: Box::new(data()),
        },
    };
    let (l, r) = tokio::join!(s.execute(&a, &c, command()), s.execute(&a, &c, command()));
    assert_eq!(usize::from(l.is_ok()) + usize::from(r.is_ok()), 1);
    assert_eq!(
        PgReceived(p.clone())
            .list(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
    sqlx::query("UPDATE membership SET role='viewer' WHERE company_id=$1::text::bigint")
        .bind(&c)
        .execute(&p)
        .await
        .unwrap();
    assert!(s.download(&a, &c, &doc.id).await.is_ok());
    assert!(s.upload(&a, &c, upload()).await.is_err());
}
#[sqlx::test(migrations = "../../migrations")]
#[ignore = "isolated PostgreSQL"]
async fn extraction_and_attach_existing_preserve_scope_and_source(p: PgPool) {
    let (a, _, c, other) = setup(&p).await;
    let s = PgInbox(p.clone());
    let item = s
        .upload(
            &a,
            &c,
            inbox::Upload {
                request_id: key(),
                filename: "synthetic.isdoc".into(),
                base64: base64::engine::general_purpose::STANDARD.encode(include_bytes!(
                    "../../../tests/fixtures/inbox/synthetic-basic.isdoc"
                )),
            },
        )
        .await
        .unwrap();
    let extracted = s.extract(&a, &c, &item.id).await.unwrap();
    assert_eq!(extracted.external_number, "SYNTHETIC-001");
    assert_eq!(extracted.lines[0].vat, "21.02");
    assert!(s.extract(&a, &other, &item.id).await.is_err());
    let target = PgReceived(p.clone())
        .create(
            &a,
            &c,
            faktura_application::received::Create {
                request_id: key(),
                data: data(),
            },
        )
        .await
        .unwrap();
    assert!(s
        .execute(
            &a,
            &other,
            inbox::Command {
                request_id: key(),
                id: item.id.clone(),
                revision: 1,
                action: inbox::Action::Attach {
                    received_id: target.id.clone()
                }
            }
        )
        .await
        .is_err());
    let linked = s
        .execute(
            &a,
            &c,
            inbox::Command {
                request_id: key(),
                id: item.id.clone(),
                revision: 1,
                action: inbox::Action::Attach {
                    received_id: target.id.clone(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(linked.received_kind.as_deref(), Some("invoice"));
    assert_eq!(linked.received_id, Some(target.id.clone()));
    let files = PgAttachment(p.clone())
        .list(&a, &c, "received", &target.id)
        .await
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].sha256, item.sha256);
    assert_eq!(
        PgReceived(p.clone())
            .list(&a, &c, Default::default())
            .await
            .unwrap()
            .total,
        1
    );
}

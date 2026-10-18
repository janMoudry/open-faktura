//! Durable quotation delivery. SMTP acceptance is not proof of recipient delivery.
use crate::{draft::authorize, sales::PgSales};
use async_trait::async_trait;
use faktura_application::{
    delivery::*,
    sales::{Document, Kind, SalesStore},
};
use lettre::{
    message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sqlx::{PgPool, Row};
use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

#[derive(Clone)]
pub struct MailConfig {
    pub sender: String,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}
impl MailConfig {
    pub fn from_env() -> std::result::Result<Option<Self>, String> {
        let Ok(host) = std::env::var("SMTP_HOST") else {
            return Ok(None);
        };
        let sender = std::env::var("SMTP_FROM").map_err(|_| "SMTP_FROM is required")?;
        let security = std::env::var("SMTP_SECURITY").unwrap_or_else(|_| "starttls".into());
        let port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| if security == "tls" { "465" } else { "587" }.into())
            .parse()
            .map_err(|_| "Invalid SMTP_PORT")?;
        let mut config = Self::builder(&host, port, &security, &sender)?;
        if let Ok(username) = std::env::var("SMTP_USERNAME") {
            let path = std::env::var("SMTP_PASSWORD_FILE")
                .map_err(|_| "SMTP_PASSWORD_FILE is required")?;
            let password =
                std::fs::read_to_string(path).map_err(|_| "Cannot read SMTP_PASSWORD_FILE")?;
            config = config.credentials(Credentials::new(
                username,
                password.trim_end_matches(['\r', '\n']).into(),
            ));
        }
        Ok(Some(Self {
            sender,
            transport: config.build(),
        }))
    }
    pub(crate) fn configured(
        host: &str,
        port: u16,
        security: &str,
        sender: &str,
        username: &str,
        password: Option<&str>,
    ) -> std::result::Result<Self, String> {
        let mut builder = Self::builder(host, port, security, sender)?;
        if !username.is_empty() {
            builder = builder.credentials(Credentials::new(
                username.into(),
                password.ok_or("Missing password")?.into(),
            ));
        }
        Ok(Self {
            sender: sender.into(),
            transport: builder.build(),
        })
    }
    pub(crate) async fn test_message(
        &self,
        to: &str,
        language: &str,
        id: &str,
    ) -> (&'static str, Option<&'static str>) {
        let subject = if language == "cs" {
            "Open Faktura — test SMTP"
        } else {
            "Open Faktura — SMTP test"
        };
        let body = if language == "cs" {
            "Testovací zpráva z Open Faktury. Nastavení SMTP umožňuje odesílání e-mailů."
        } else {
            "Test message from Open Faktura. Your SMTP settings allow sending email."
        };
        let message = Message::builder()
            .from(match self.sender.parse() {
                Ok(v) => v,
                Err(_) => return ("failed", Some("message_invalid")),
            })
            .to(match to.parse() {
                Ok(v) => v,
                Err(_) => return ("failed", Some("message_invalid")),
            })
            .message_id(Some(format!("{id}@open-faktura.local")))
            .subject(subject)
            .body(body.to_owned());
        let Ok(message) = message else {
            return ("failed", Some("message_invalid"));
        };
        match tokio::time::timeout(Duration::from_secs(45), self.transport.send(message)).await {
            Ok(Ok(_)) => ("sent", None),
            Ok(Err(e)) if e.is_permanent() || e.is_transient() => ("failed", Some("smtp_rejected")),
            _ => ("uncertain", Some("smtp_unknown")),
        }
    }
    fn builder(
        host: &str,
        port: u16,
        security: &str,
        sender: &str,
    ) -> std::result::Result<lettre::transport::smtp::AsyncSmtpTransportBuilder, String> {
        sender
            .parse::<lettre::Address>()
            .map_err(|_| "Invalid SMTP_FROM")?;
        let builder = match security {
            "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .map_err(|_| "Invalid SMTP_HOST")?,
            "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                .map_err(|_| "Invalid SMTP_HOST")?,
            "plain" if ["127.0.0.1", "::1", "localhost"].contains(&host) => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            }
            _ => {
                return Err(
                    "SMTP_SECURITY must be starttls/tls; plain is restricted to loopback".into(),
                )
            }
        };
        Ok(builder.port(port).timeout(Some(Duration::from_secs(30))))
    }
    /// Local SMTP capture for integration tests, never resolves a remote address.
    pub fn local_capture(port: u16) -> Self {
        Self {
            sender: "sender@example.test".into(),
            transport: Self::builder("127.0.0.1", port, "plain", "sender@example.test")
                .expect("local SMTP")
                .build(),
        }
    }
}
#[derive(Clone)]
pub struct PgDelivery {
    pub pool: PgPool,
    pub mail: Option<Arc<MailConfig>>,
}
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
pub(crate) fn map(r: sqlx::postgres::PgRow) -> Delivery {
    Delivery {
        purpose: r.try_get("purpose").unwrap_or_else(|_| "document".into()),
        attachments: serde_json::from_value(r.get("attachments"))
            .expect("persisted attachment metadata"),
        id: r.get::<i64, _>("id").to_string(),
        document_id: r.get::<i64, _>("document_id").to_string(),
        revision: r.get("revision"),
        actor_id: r.get::<i64, _>("actor_id").to_string(),
        share_id: r.get::<Option<i64>, _>("share_id").map(|v| v.to_string()),
        sender: r.get("sender"),
        reply_to: r.get("reply_to"),
        recipient: r.get("recipient"),
        subject: r.get("subject"),
        body: r.get("body"),
        status: r.get("status"),
        error_code: r.get("error_code"),
        created_at: r
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
        finished_at: r
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
            .map(|v| v.to_rfc3339()),
    }
}
static RENDER: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(2)));
pub(crate) async fn render(v: Document) -> Result<Vec<u8>> {
    if v.kind != Kind::Quote {
        return Err(Error::NotFound);
    }
    faktura_application::sales::validate(&v.data, &v.kind)?;
    if v.totals.is_none() || v.data.invoice.customer.is_none() {
        return Err(Error::Invalid(vec!["document".into()]));
    }
    let permit = RENDER.clone().try_acquire_owned().map_err(err)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut data = v.data.invoice;
        // iDoklad internal notes are private; the quotation description is public.
        data.note = v.data.name;
        let style = v.document_style;
        crate::invoice_pdf::render_quote_with_style(
            &faktura_application::draft::Draft {
                advances: None,
                id: v.id,
                company_id: v.company_id,
                revision: v.revision,
                data,
                totals: v.totals,
                number: v.number,
                settlement: None,
                issued_at: None,
            },
            style.as_ref(),
        )
        .map_err(err)
    })
    .await
    .map_err(err)?
}
#[async_trait]
impl DeliveryStore for PgDelivery {
    async fn pdf(&self, a: &str, c: &str, d: &str, delivery: Option<&str>) -> Result<Vec<u8>> {
        let quote = PgSales(self.pool.clone()).get(a, c, d).await?;
        if quote.kind != Kind::Quote {
            return Err(Error::NotFound);
        }
        if let Some(delivery) = delivery {
            let mut tx = self.pool.begin().await.map_err(err)?;
            let (_, c) = authorize(&mut tx, a, c, false).await?;
            return sqlx::query_scalar(
                "SELECT pdf FROM quote_delivery WHERE company_id=$1 AND document_id=$2 AND id=$3",
            )
            .bind(c)
            .bind(id(d)?)
            .bind(id(delivery)?)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound);
        }
        render(quote).await
    }
    async fn history(&self, a: &str, c: &str, d: &str, page: i64) -> Result<History> {
        let quote = PgSales(self.pool.clone()).get(a, c, d).await?;
        if quote.kind != Kind::Quote {
            return Err(Error::NotFound);
        }
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let total = sqlx::query_scalar(
            "SELECT count(*) FROM quote_delivery WHERE company_id=$1 AND document_id=$2",
        )
        .bind(c)
        .bind(id(d)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items=sqlx::query("SELECT id,document_id,revision,actor_id,share_id,attachments,sender,reply_to,recipient,subject,body,status,error_code,created_at,finished_at FROM quote_delivery WHERE company_id=$1 AND document_id=$2 ORDER BY id DESC LIMIT 25 OFFSET $3").bind(c).bind(id(d)?).bind((page-1)*25).fetch_all(&mut *tx).await.map_err(err)?.into_iter().map(map).collect();
        let mail = crate::smtp::resolve(&self.pool, self.mail.clone()).await?;
        Ok(History {
            configured: mail.is_some(),
            sender: mail.as_ref().map(|v| v.sender.clone()),
            items,
            total,
            page,
        })
    }
    async fn send(&self, a: &str, c: &str, d: &str, cmd: SendQuote) -> Result<Delivery> {
        if cmd.purpose.is_some()
            || cmd.settlement_revision.is_some()
            || cmd.automation_revision.is_some()
        {
            return Err(Error::Invalid(vec!["purpose".into()]));
        }
        cmd.recipient
            .parse::<lettre::Address>()
            .map_err(|_| Error::Invalid(vec!["recipient".into()]))?;
        let mut command = serde_json::to_value(&cmd).map_err(err)?;
        // Preserve idempotency comparisons for commands persisted before link delivery existed.
        if cmd.share_id.is_none() {
            command
                .as_object_mut()
                .ok_or(Error::Unavailable)?
                .remove("share_id");
        }
        // Replay can succeed even when the quote changed or SMTP is temporarily unconfigured.
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (actor, company) = authorize(&mut tx, a, c, true).await?;
        let replay = sqlx::query(
            "SELECT * FROM quote_delivery WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(company)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
        if let Some(r) = replay {
            if r.get::<i64, _>("actor_id") != actor
                || r.get::<i64, _>("document_id") != id(d)?
                || r.get::<serde_json::Value, _>("command") != command
            {
                return Err(Error::Conflict);
            }
            return Ok(map(r));
        }
        tx.commit().await.map_err(err)?;
        let mail = crate::smtp::resolve(&self.pool, self.mail.clone())
            .await?
            .ok_or_else(|| Error::Invalid(vec!["smtp".into()]))?;
        let quote = PgSales(self.pool.clone()).get(a, c, d).await?;
        if quote.archived || quote.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        let snapshot = serde_json::to_value(&quote).map_err(err)?;
        let reply_to = quote.data.invoice.supplier.email.clone();
        if !reply_to.is_empty() {
            reply_to
                .parse::<lettre::Address>()
                .map_err(|_| Error::Invalid(vec!["supplier.email".into()]))?;
        }
        let pdf = render(quote).await?;
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (actor, company) = authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(company)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM quote_delivery WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(company)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != actor
                || r.get::<i64, _>("document_id") != id(d)?
                || r.get::<serde_json::Value, _>("command") != command
            {
                return Err(Error::Conflict);
            }
            return Ok(map(r));
        }
        let link_url: Option<String> = if let Some(share) = &cmd.share_id {
            Some(sqlx::query_scalar("SELECT url FROM quote_share WHERE company_id=$1 AND document_id=$2 AND id=$3 AND revoked_at IS NULL AND expires_at>now() FOR SHARE").bind(company).bind(id(d)?).bind(id(share)?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?)
        } else {
            None
        };
        let row =
            sqlx::query("SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(company)
                .bind(id(d)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?;
        crate::sales::ensure_not_trashed(&mut tx, company, id(d)?).await?;
        let current = crate::sales::map(row)?;
        if current.revision != cmd.revision || current.archived {
            return Err(Error::Conflict);
        }
        let message_id = format!(
            "{}@{}",
            uuid::Uuid::new_v4(),
            mail.sender.split('@').next_back().unwrap_or("localhost")
        );
        let body = if let Some(url) = link_url {
            format!("{}\n\n{}", cmd.body, url)
        } else {
            cmd.body
        };
        let files =
            crate::attachment::snapshot(&mut tx, company, "quote", id(d)?, &cmd.attachment_ids)
                .await?;
        let row=sqlx::query("INSERT INTO quote_delivery(company_id,document_id,actor_id,request_id,command,revision,snapshot,pdf,sender,reply_to,recipient,subject,body,message_id,share_id,attachments) VALUES($1,$2,$3,$4::text::uuid,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING *").bind(company).bind(id(d)?).bind(actor).bind(&cmd.request_id).bind(command).bind(cmd.revision).bind(snapshot).bind(pdf).bind(&mail.sender).bind(reply_to).bind(cmd.recipient).bind(cmd.subject).bind(body).bind(message_id).bind(cmd.share_id.as_deref().map(id).transpose()?).bind(serde_json::to_value(&files).map_err(err)?).fetch_one(&mut *tx).await.map_err(err)?;
        crate::attachment::link(&mut tx, company, "quote", row.get("id"), &files).await?;
        tx.commit().await.map_err(err)?;
        Ok(map(row))
    }
}
impl PgDelivery {
    /// One bounded attempt. Uncertain attempts are never retried automatically.
    pub async fn process_one(&self) -> Result<bool> {
        let Some(mail) = crate::smtp::resolve(&self.pool, self.mail.clone()).await? else {
            return Ok(false);
        };
        // Lease exceeds the entire SMTP deadline; this also recovers crashed workers.
        sqlx::query("UPDATE quote_delivery SET status='uncertain',error_code='interrupted',finished_at=now() WHERE status='sending' AND started_at < now()-interval '2 minutes'").execute(&self.pool).await.map_err(err)?;
        sqlx::query("UPDATE quote_delivery d SET status='failed',error_code='link_unavailable',finished_at=now() WHERE d.status='queued' AND d.share_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM quote_share l JOIN sales_document q ON q.id=l.document_id AND q.company_id=l.company_id JOIN company c ON c.id=l.company_id WHERE l.id=d.share_id AND l.revoked_at IS NULL AND l.expires_at>now() AND NOT q.archived AND c.deleted_at IS NULL)").execute(&self.pool).await.map_err(err)?;
        let row=sqlx::query("UPDATE quote_delivery SET status='sending',started_at=now() WHERE id=(SELECT id FROM quote_delivery WHERE status='queued' ORDER BY id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING *").fetch_optional(&self.pool).await.map_err(err)?;
        let Some(row) = row else { return Ok(false) };
        let delivery_id: i64 = row.get("id");
        let company: i64 = row.get("company_id");
        let document: i64 = row.get("document_id");
        let actor: i64 = row.get("actor_id");
        let revision: i64 = row.get("revision");
        let files =
            crate::attachment::queued(&self.pool, row.get("company_id"), "quote", row.get("id"))
                .await?;
        let (status, code) = mail.deliver(&row, files).await;
        let mut tx = self.pool.begin().await.map_err(err)?;
        sqlx::query("UPDATE quote_delivery SET status=$2,error_code=$3,finished_at=now() WHERE id=$1 AND status='sending'").bind(delivery_id).bind(status).bind(code).execute(&mut *tx).await.map_err(err)?;
        let row =
            sqlx::query("SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(company)
                .bind(document)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        let mut quote = crate::sales::map(row)?;
        if status == "sent"
            && quote.revision == revision
            && quote.data.status == "created"
            && !quote.archived
            && quote.invoice_id.is_none()
        {
            quote.data.status = "sent".into();
            quote.revision += 1;
            sqlx::query("UPDATE sales_document SET data=$3,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(company).bind(document).bind(serde_json::to_value(&quote.data).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
            crate::sales::audit(&mut tx, actor, &quote, "email_sent").await?;
        }
        tx.commit().await.map_err(err)?;
        Ok(true)
    }
}

impl MailConfig {
    pub(crate) async fn deliver(
        &self,
        row: &sqlx::postgres::PgRow,
        files: Vec<faktura_application::attachment::File>,
    ) -> (&'static str, Option<&'static str>) {
        let build =
            (|| -> std::result::Result<Message, Box<dyn std::error::Error + Send + Sync>> {
                let mut msg = Message::builder()
                    .from(row.get::<String, _>("sender").parse::<Mailbox>()?)
                    .to(row.get::<String, _>("recipient").parse::<Mailbox>()?)
                    .subject(row.get::<String, _>("subject"))
                    .message_id(Some(row.get("message_id")));
                let reply: String = row.get("reply_to");
                if !reply.is_empty() {
                    msg = msg.reply_to(reply.parse()?);
                }
                if row.get::<Option<i64>, _>("share_id").is_some() && files.is_empty() {
                    return Ok(msg.singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(row.get::<String, _>("body")),
                    )?);
                }
                let mut mixed = MultiPart::mixed().singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(row.get::<String, _>("body")),
                );
                if row.get::<Option<i64>, _>("share_id").is_none() {
                    let snapshot: serde_json::Value = row.get("snapshot");
                    mixed = mixed.singlepart(
                        Attachment::new(format!(
                            "{}.pdf",
                            snapshot["number"]
                                .as_str()
                                .ok_or("missing document number")?
                        ))
                        .body(row.get::<Vec<u8>, _>("pdf"), "application/pdf".parse()?),
                    );
                }
                for file in files {
                    mixed = mixed.singlepart(
                        Attachment::new(file.metadata.filename)
                            .body(file.content, file.metadata.media_type.parse()?),
                    );
                }
                Ok(msg.multipart(mixed)?)
            })();
        match build {
            Err(_) => ("failed", Some("message_invalid")),
            Ok(message) => self.dispatch_message(message).await,
        }
    }
}

impl MailConfig {
    pub(crate) async fn dispatch_message(
        &self,
        message: Message,
    ) -> (&'static str, Option<&'static str>) {
        match tokio::time::timeout(Duration::from_secs(45), self.transport.send(message)).await {
            Ok(Ok(_)) => ("sent", None),
            Ok(Err(e)) if e.is_permanent() || e.is_transient() => ("failed", Some("smtp_rejected")),
            _ => ("uncertain", Some("smtp_unknown")),
        }
    }
}

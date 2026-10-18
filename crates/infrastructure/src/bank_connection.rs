#[path = "bank_secret.rs"]
mod secret;
#[path = "bank_transport.rs"]
pub mod transport;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use faktura_application::{bank_connection::*, banking::StatementInput};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
#[derive(Clone)]
pub struct PgConnection {
    pub pool: PgPool,
    pub transport: Arc<dyn transport::Transport>,
}
impl PgConnection {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            transport: Arc::new(transport::HttpTransport),
        }
    }
}
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn invalid(s: &str) -> Error {
    Error::Invalid(vec![s.into()])
}
async fn lock(tx: &mut Transaction<'_, Postgres>, c: i64) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok(())
}
fn run(r: sqlx::postgres::PgRow) -> Run {
    let status: String = r.get("status");
    let stale: bool = r.get("stale");
    Run {
        kind: r.get("kind"),
        status: if stale { "failed".into() } else { status },
        started_at: r
            .get::<chrono::DateTime<chrono::Utc>, _>("started_at")
            .to_rfc3339(),
        finished_at: r
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("finished_at")
            .map(|v| v.to_rfc3339()),
        error_code: if stale {
            Some("connection_interrupted".into())
        } else {
            r.get("error_code")
        },
        imported: r.get("imported"),
        duplicates: r.get("duplicates"),
    }
}
async fn status(tx: &mut Transaction<'_, Postgres>, c: i64, id: i64) -> Result<Status> {
    let row = sqlx::query(
        "SELECT revision,provider,schedule_enabled,interval_minutes,next_run_at,schedule_error,matching_enabled,matching_treasury,matching_next_at,matching_error FROM bank_connection WHERE company_id=$1 AND account_id=$2",
    )
    .bind(c)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?;
    let last=sqlx::query("SELECT *,status='running' AND started_at<now()-interval '60 seconds' AS stale FROM bank_connection_run WHERE company_id=$1 AND account_id=$2 ORDER BY started_at DESC LIMIT 1").bind(c).bind(id).fetch_optional(&mut **tx).await.map_err(err)?.map(run);
    let last_sync:Option<chrono::DateTime<chrono::Utc>>=sqlx::query_scalar("SELECT max(finished_at) FROM bank_connection_run WHERE company_id=$1 AND account_id=$2 AND kind='sync' AND status='success'").bind(c).bind(id).fetch_one(&mut **tx).await.map_err(err)?;
    let provider = row
        .as_ref()
        .and_then(|r| r.get::<Option<String>, _>("provider"));
    let matching_last=sqlx::query("SELECT created_at,checked,matched FROM bank_matching_run WHERE company_id=$1 AND account_id=$2 ORDER BY id DESC LIMIT 1").bind(c).bind(id).fetch_optional(&mut **tx).await.map_err(err)?.map(|r|MatchingRun{at:r.get::<chrono::DateTime<chrono::Utc>,_>("created_at").to_rfc3339(),checked:r.get("checked"),matched:r.get("matched")});
    Ok(Status {
        matching_enabled: row.as_ref().is_some_and(|r| r.get("matching_enabled")),
        matching_treasury: row.as_ref().is_some_and(|r| r.get("matching_treasury")),
        matching_next_at: row
            .as_ref()
            .and_then(|r| r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("matching_next_at"))
            .map(|v| v.to_rfc3339()),
        matching_error: row.as_ref().and_then(|r| r.get("matching_error")),
        matching_last,
        schedule_enabled: row.as_ref().is_some_and(|r| r.get("schedule_enabled")),
        interval_minutes: row.as_ref().map_or(60, |r| r.get("interval_minutes")),
        next_run_at: row
            .as_ref()
            .and_then(|r| r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("next_run_at"))
            .map(|v| v.to_rfc3339()),
        schedule_error: row.as_ref().and_then(|r| r.get("schedule_error")),
        revision: row.map_or(0, |r| r.get("revision")),
        configured: provider.is_some(),
        provider,
        busy: last.as_ref().is_some_and(|r| r.status == "running"),
        last_run: last,
        last_sync_at: last_sync.map(|v| v.to_rfc3339()),
        providers: transport::endpoints()?
            .into_iter()
            .map(|v| v.provider)
            .collect(),
    })
}
#[async_trait]
impl ConnectionStore for PgConnection {
    async fn status(&self, a: &str, c: &str, account: &str) -> Result<Status> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        crate::banking::get_account(&mut tx, c, account).await?;
        status(&mut tx, c, account.parse().map_err(err)?).await
    }
    async fn execute(&self, a: &str, company: &str, cmd: Command) -> Result<Status> {
        if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
            return Err(invalid("request_id"));
        }
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (actor, c) = crate::draft::authorize(&mut tx, a, company, true).await?;
        lock(&mut tx, c).await?;
        let account = crate::banking::get_account(&mut tx, c, &cmd.account_id).await?;
        let id: i64 = account.id.parse().map_err(err)?;
        let kind = match &cmd.action {
            Action::Save { .. } => "save",
            Action::Matching { .. } => "matching",
            Action::Schedule { .. } => "schedule",
            Action::Disconnect => "disconnect",
            Action::Test => "test",
            Action::Sync => "sync",
        };
        // A save retry must not persist or compare plaintext tokens; optimistic revision protects it.
        if let Some(previous)=sqlx::query("SELECT kind,actor_id,revision FROM bank_connection_run WHERE company_id=$1 AND account_id=$2 AND request_id=$3::text::uuid").bind(c).bind(id).bind(&cmd.request_id).fetch_optional(&mut *tx).await.map_err(err)? {
   if previous.get::<String,_>("kind")!=kind || previous.get::<i64,_>("actor_id")!=actor || previous.get::<i64,_>("revision")!=cmd.revision || kind=="save" || kind=="schedule" || kind=="matching" {return Err(Error::Conflict);}
   return status(&mut tx,c,id).await;
  }
        let current = status(&mut tx, c, id).await?;
        if current.revision != cmd.revision
            || current.busy
            || (account.archived
                && !matches!(
                    &cmd.action,
                    Action::Disconnect | Action::Matching { enabled: false, .. }
                ))
        {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO bank_connection(company_id,account_id) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(c).bind(id).execute(&mut *tx).await.map_err(err)?;
        let context = format!("open-faktura-bank-v1:{c}:{id}");
        if let Action::Save {
            provider,
            secret: input,
        } = &cmd.action
        {
            if !transport::endpoints()?
                .iter()
                .any(|p| p.provider.id == *provider)
            {
                return Err(invalid("connection_provider"));
            }
            let encrypted = if let Some(input) = input {
                if input.is_empty()
                    || input.len() > 2048
                    || input.chars().any(char::is_control)
                    || (provider == "fio"
                        && (input.len() != 64 || !input.bytes().all(|v| v.is_ascii_alphanumeric())))
                {
                    return Err(invalid("connection_secret"));
                }
                secret::encrypt(input, &context)?
            } else {
                if current.provider.as_deref() != Some(provider) {
                    return Err(invalid("connection_secret"));
                }
                sqlx::query_scalar::<_, Vec<u8>>(
                    "SELECT secret FROM bank_connection WHERE company_id=$1 AND account_id=$2",
                )
                .bind(c)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?
            };
            sqlx::query("UPDATE bank_connection SET provider=$3,secret=$4,schedule_enabled=false,next_run_at=NULL,schedule_error=NULL,revision=revision+1,updated_by=$5,updated_at=now() WHERE company_id=$1 AND account_id=$2").bind(c).bind(id).bind(provider).bind(encrypted).bind(actor).execute(&mut *tx).await.map_err(err)?;
        } else if matches!(cmd.action, Action::Disconnect) {
            sqlx::query("UPDATE bank_connection SET provider=NULL,secret=NULL,schedule_enabled=false,next_run_at=NULL,schedule_error=NULL,revision=revision+1,updated_by=$3,updated_at=now() WHERE company_id=$1 AND account_id=$2").bind(c).bind(id).bind(actor).execute(&mut *tx).await.map_err(err)?;
        } else if !current.configured && kind != "matching" {
            return Err(invalid("connection_missing"));
        }
        if let Action::Schedule {
            enabled,
            interval_minutes,
        } = cmd.action
        {
            if ![15, 60, 360, 1440].contains(&interval_minutes) {
                return Err(invalid("connection_interval"));
            }
            sqlx::query("UPDATE bank_connection SET schedule_enabled=$3,interval_minutes=$4,schedule_actor_id=$5,next_run_at=CASE WHEN $3 THEN now()+make_interval(mins=>$4) ELSE NULL END,schedule_error=NULL,revision=revision+1,updated_by=$5,updated_at=now() WHERE company_id=$1 AND account_id=$2").bind(c).bind(id).bind(enabled).bind(interval_minutes).bind(actor).execute(&mut *tx).await.map_err(err)?;
        }
        if let Action::Matching {
            enabled,
            include_treasury,
        } = cmd.action
        {
            sqlx::query("UPDATE bank_connection SET matching_enabled=$3,matching_treasury=$5,matching_actor_id=$4,matching_next_at=CASE WHEN $3 THEN now()+interval '30 seconds' ELSE NULL END,matching_cursor=0,matching_error=NULL,revision=revision+1,updated_by=$4,updated_at=now() WHERE company_id=$1 AND account_id=$2").bind(c).bind(id).bind(enabled).bind(actor).bind(include_treasury).execute(&mut *tx).await.map_err(err)?;
        }
        let io = kind == "test" || kind == "sync";
        sqlx::query("INSERT INTO bank_connection_run(company_id,account_id,request_id,actor_id,revision,kind,status,finished_at) VALUES($1,$2,$3::text::uuid,$4,$5,$6,$7,CASE WHEN $7='success' THEN now() ELSE NULL END)").bind(c).bind(id).bind(&cmd.request_id).bind(actor).bind(cmd.revision).bind(kind).bind(if io{"running"}else{"success"}).execute(&mut *tx).await.map_err(err)?;
        if !io {
            let out = status(&mut tx, c, id).await?;
            tx.commit().await.map_err(err)?;
            return Ok(out);
        }
        let encrypted: Vec<u8> = sqlx::query_scalar(
            "SELECT secret FROM bank_connection WHERE company_id=$1 AND account_id=$2",
        )
        .bind(c)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let token = secret::decrypt(&encrypted, &context)?;
        let ep = transport::endpoints()?
            .into_iter()
            .find(|v| Some(&v.provider.id) == current.provider.as_ref())
            .ok_or_else(|| invalid("connection_provider"))?;
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        let from = if kind == "test" {
            today
        } else {
            today - chrono::Duration::days(29)
        };
        tx.commit().await.map_err(err)?;
        // No database locks held during external I/O. A bounded lease protects config and commits.
        let fetched = self
            .transport
            .fetch(
                &ep,
                &token,
                &account.data.iban,
                &from.to_string(),
                &today.to_string(),
            )
            .await;
        let parsed = fetched.and_then(|bytes| {
            crate::bank_statement::parse_connection(
                &StatementInput {
                    account_id: account.id.clone(),
                    format: ep.format.clone(),
                    filename: "connection.xml".into(),
                    content_base64: STANDARD.encode(bytes),
                },
                &account.data.iban,
                &account.data.currency,
            )
            .map_err(|e| match e {
                Error::Invalid(v) if v.iter().any(|f| f == "statement_account") => {
                    "connection_account".into()
                }
                _ => "connection_format".into(),
            })
        });
        let mut tx = self.pool.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, company, true).await?;
        lock(&mut tx, c).await?;
        let fresh = crate::banking::get_account(&mut tx, c, &account.id).await?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bank_connection b JOIN bank_connection_run r USING(company_id,account_id) WHERE b.company_id=$1 AND b.account_id=$2 AND b.revision=$3 AND r.request_id=$4::text::uuid AND r.status='running' AND r.started_at>now()-interval '60 seconds')").bind(c).bind(id).bind(cmd.revision).bind(&cmd.request_id).fetch_one(&mut *tx).await.map_err(err)?;
        if !valid || fresh.archived || fresh.data.iban != account.data.iban {
            return Err(Error::Conflict);
        }
        let mut imported = 0;
        let mut duplicates = 0;
        let error = match parsed {
            Err(e) => Some(e),
            Ok(rows) => {
                if rows
                    .iter()
                    .any(|r| r.booked_on < from || r.booked_on > today)
                {
                    Some("connection_format".into())
                } else if kind == "test" {
                    None
                } else {
                    match crate::banking::preview_rows(&mut tx, c, fresh, rows, &ep.format, 1).await
                    {
                        Ok(report) if report.valid => {
                            for row in report.rows.iter().filter(|r| r.status == "new") {
                                crate::banking::insert_movement(
                                    &mut tx,
                                    c,
                                    &account.id,
                                    &row.data,
                                    &ep.format,
                                )
                                .await?;
                            }
                            imported = report.new_count as i32;
                            duplicates = report.duplicate_count as i32;
                            None
                        }
                        Ok(_) => Some("connection_conflict".into()),
                        Err(Error::Invalid(_)) => Some("connection_format".into()),
                        Err(e) => return Err(e),
                    }
                }
            }
        };
        sqlx::query("UPDATE bank_connection_run SET status=$4,error_code=$5,imported=$6,duplicates=$7,finished_at=now() WHERE company_id=$1 AND account_id=$2 AND request_id=$3::text::uuid").bind(c).bind(id).bind(&cmd.request_id).bind(if error.is_some(){"failed"}else{"success"}).bind(error).bind(imported).bind(duplicates).execute(&mut *tx).await.map_err(err)?;
        let out = status(&mut tx, c, id).await?;
        tx.commit().await.map_err(err)?;
        Ok(out)
    }
}

impl PgConnection {
    /// One bounded scheduled job, shared by all API instances. No network under SQL locks.
    pub async fn process_one(&self) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(err)?;
        let candidate=sqlx::query("SELECT company_id,account_id,schedule_actor_id,revision FROM bank_connection WHERE schedule_enabled AND next_run_at<=now() ORDER BY next_run_at,company_id,account_id FOR UPDATE SKIP LOCKED LIMIT 1").fetch_optional(&mut *tx).await.map_err(err)?;
        let Some(row) = candidate else {
            return Ok(false);
        };
        let c: i64 = row.get("company_id");
        let id: i64 = row.get("account_id");
        let actor: i64 = row.get("schedule_actor_id");
        let revision: i64 = row.get("revision");
        let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bank_account b JOIN company c ON c.id=b.company_id JOIN membership m ON m.company_id=c.id WHERE b.company_id=$1 AND b.id=$2 AND NOT b.archived AND c.deleted_at IS NULL AND m.user_id=$3 AND m.role IN ('owner','editor'))").bind(c).bind(id).bind(actor).fetch_one(&mut *tx).await.map_err(err)?;
        if !allowed {
            sqlx::query("UPDATE bank_connection SET schedule_enabled=false,next_run_at=NULL,schedule_error='connection_permission' WHERE company_id=$1 AND account_id=$2").bind(c).bind(id).execute(&mut *tx).await.map_err(err)?;
            tx.commit().await.map_err(err)?;
            return Ok(true);
        }
        sqlx::query("UPDATE bank_connection SET next_run_at=now()+make_interval(mins=>interval_minutes),schedule_error=NULL WHERE company_id=$1 AND account_id=$2").bind(c).bind(id).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        let result = self
            .execute(
                &actor.to_string(),
                &c.to_string(),
                Command {
                    account_id: id.to_string(),
                    revision,
                    request_id: uuid::Uuid::new_v4().to_string(),
                    action: Action::Sync,
                },
            )
            .await;
        if let Err(error) = result {
            let lost = matches!(error, Error::Forbidden | Error::NotFound);
            let code = if lost {
                "connection_permission"
            } else if matches!(error, Error::Conflict) {
                "connection_busy"
            } else {
                "connection_setup"
            };
            sqlx::query("UPDATE bank_connection SET schedule_error=$4,schedule_enabled=CASE WHEN $5 THEN false ELSE schedule_enabled END,next_run_at=CASE WHEN $5 THEN NULL ELSE next_run_at END WHERE company_id=$1 AND account_id=$2 AND revision=$3 AND schedule_enabled").bind(c).bind(id).bind(revision).bind(code).bind(lost).execute(&self.pool).await.map_err(err)?;
        }
        Ok(true)
    }
}

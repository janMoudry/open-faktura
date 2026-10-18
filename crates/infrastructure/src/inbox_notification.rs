use async_trait::async_trait;
use faktura_application::inbox_notification::*;
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgNotifications(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
async fn prepare(tx: &mut Transaction<'_, Postgres>, a: &str, c: &str) -> Result<(i64, i64)> {
    let (a, c) = crate::draft::authorize(tx, a, c, false).await?;
    // Serialize the user's settings and read updates; company lock stabilizes feed
    // against event triggers and keeps event identities ordered through commit.
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok((a, c))
}
async fn load(tx: &mut Transaction<'_, Postgres>, a: i64, c: i64) -> Result<Feed> {
    let s =
        sqlx::query("SELECT * FROM inbox_notification_settings WHERE company_id=$1 AND user_id=$2")
            .bind(c)
            .bind(a)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?;
    let Some(s) = s else {
        return Ok(Feed {
            settings: Settings {
                revision: 1,
                new_documents: false,
                errors: false,
            },
            events: vec![],
            unread: 0,
            through: "0".into(),
        });
    };
    let settings = Settings {
        revision: s.get("revision"),
        new_documents: s.get("new_documents"),
        errors: s.get("errors"),
    };
    let dr: i64 = s.get("documents_read");
    let er: i64 = s.get("errors_read");
    let rows=sqlx::query("SELECT *, CASE WHEN category='document' THEN id>$4 ELSE id>$5 END AS unread FROM inbox_notification_event WHERE company_id=$1 AND ((category='document' AND $2) OR (category='error' AND $3)) ORDER BY id DESC LIMIT 50").bind(c).bind(settings.new_documents).bind(settings.errors).bind(dr).bind(er).fetch_all(&mut **tx).await.map_err(err)?;
    let unread:i64=sqlx::query_scalar("SELECT count(*) FROM inbox_notification_event WHERE company_id=$1 AND ((category='document' AND $2 AND id>$4) OR (category='error' AND $3 AND id>$5))").bind(c).bind(settings.new_documents).bind(settings.errors).bind(dr).bind(er).fetch_one(&mut **tx).await.map_err(err)?;
    let through = rows
        .first()
        .map(|r| r.get::<i64, _>("id"))
        .unwrap_or(0)
        .to_string();
    let events = rows
        .iter()
        .map(|r| Event {
            id: r.get::<i64, _>("id").to_string(),
            category: r.get("category"),
            document_id: r
                .get::<Option<i64>, _>("document_id")
                .map(|v| v.to_string()),
            filename: r.get("filename"),
            code: r.get("code"),
            created_at: r
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .to_rfc3339(),
            unread: r.get("unread"),
        })
        .collect();
    Ok(Feed {
        settings,
        events,
        unread,
        through,
    })
}
#[async_trait]
impl NotificationsStore for PgNotifications {
    async fn feed(&self, a: &str, c: &str) -> Result<Feed> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = prepare(&mut tx, a, c).await?;
        let f = load(&mut tx, a, c).await?;
        tx.commit().await.map_err(err)?;
        Ok(f)
    }
    async fn configure(&self, a: &str, c: &str, v: Configure) -> Result<Feed> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = prepare(&mut tx, a, c).await?;
        sqlx::query("INSERT INTO inbox_notification_settings(company_id,user_id) VALUES($1,$2) ON CONFLICT DO NOTHING").bind(c).bind(a).execute(&mut *tx).await.map_err(err)?;
        // Enabling starts at current position, avoiding unsolicited historical alerts.
        let changed=sqlx::query("UPDATE inbox_notification_settings SET revision=revision+1,documents_read=CASE WHEN $4 AND NOT new_documents THEN (SELECT coalesce(max(id),0) FROM inbox_notification_event WHERE company_id=$1) ELSE documents_read END,errors_read=CASE WHEN $5 AND NOT errors THEN (SELECT coalesce(max(id),0) FROM inbox_notification_event WHERE company_id=$1) ELSE errors_read END,new_documents=$4,errors=$5 WHERE company_id=$1 AND user_id=$2 AND revision=$3").bind(c).bind(a).bind(v.revision).bind(v.new_documents).bind(v.errors).execute(&mut *tx).await.map_err(err)?.rows_affected();
        if changed == 0 {
            return Err(Error::Conflict);
        }
        let f = load(&mut tx, a, c).await?;
        tx.commit().await.map_err(err)?;
        Ok(f)
    }
    async fn read(&self, a: &str, c: &str, through: &str) -> Result<Feed> {
        let n = through
            .parse::<i64>()
            .map_err(|_| Error::Invalid(vec!["through".into()]))?;
        if n < 0 {
            return Err(Error::Invalid(vec!["through".into()]));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = prepare(&mut tx, a, c).await?;
        if n!=0 && !sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM inbox_notification_event e JOIN inbox_notification_settings s ON s.company_id=e.company_id AND s.user_id=$2 WHERE e.company_id=$1 AND e.id=$3 AND ((e.category='document' AND s.new_documents) OR (e.category='error' AND s.errors)))").bind(c).bind(a).bind(n).fetch_one(&mut *tx).await.map_err(err)?{return Err(Error::NotFound)}
        sqlx::query("UPDATE inbox_notification_settings SET documents_read=CASE WHEN new_documents THEN greatest(documents_read,$3) ELSE documents_read END,errors_read=CASE WHEN errors THEN greatest(errors_read,$3) ELSE errors_read END WHERE company_id=$1 AND user_id=$2").bind(c).bind(a).bind(n).execute(&mut *tx).await.map_err(err)?;
        let f = load(&mut tx, a, c).await?;
        tx.commit().await.map_err(err)?;
        Ok(f)
    }
}

/// Called only after gateway intake rollback. No submitted content is persisted.
pub(crate) async fn gateway_failure(pool: &PgPool, a: &str, c: &str, request: &str) -> Result<()> {
    let key =
        uuid::Uuid::parse_str(request).map_err(|_| Error::Invalid(vec!["request_id".into()]))?;
    let mut tx = pool.begin().await.map_err(err)?;
    let (_, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(c)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
    let enabled: bool = sqlx::query_scalar(
        "SELECT coalesce((SELECT email_enabled FROM inbox_settings WHERE company_id=$1),false)",
    )
    .bind(c)
    .fetch_one(&mut *tx)
    .await
    .map_err(err)?;
    if !enabled {
        return Err(Error::Forbidden);
    }
    sqlx::query("INSERT INTO inbox_notification_event(company_id,category,code,source_key) VALUES($1,'error','gateway_failed',$2) ON CONFLICT(company_id,source_key) WHERE source_key IS NOT NULL DO NOTHING").bind(c).bind(key).execute(&mut *tx).await.map_err(err)?;
    tx.commit().await.map_err(err)?;
    Ok(())
}

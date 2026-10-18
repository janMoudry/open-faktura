//! Conservative opt-in matching; shares the manual batch evaluator and payment commands.
use super::*;
impl PgBank {
    pub async fn process_matching_one(&self) -> Result<bool> {
        let row=sqlx::query("SELECT company_id,account_id,matching_actor_id,revision FROM bank_connection WHERE matching_enabled AND matching_next_at<=now() ORDER BY matching_next_at,company_id,account_id LIMIT 1").fetch_optional(&self.0).await.map_err(err)?;
        let Some(row) = row else { return Ok(false) };
        let c: i64 = row.get("company_id");
        let account: i64 = row.get("account_id");
        let actor: i64 = row.get("matching_actor_id");
        let revision: i64 = row.get("revision");
        match self.matching_batch(c, account, actor, revision).await {
            Ok(v) => Ok(v),
            Err(error) => {
                let pause = matches!(error, Error::Forbidden | Error::NotFound);
                sqlx::query("UPDATE bank_connection SET matching_error=$4,matching_enabled=CASE WHEN $5 THEN false ELSE matching_enabled END,matching_next_at=CASE WHEN $5 THEN NULL ELSE now()+interval '5 minutes' END WHERE company_id=$1 AND account_id=$2 AND revision=$3 AND matching_enabled").bind(c).bind(account).bind(revision).bind(if pause{"matching_permission"}else{"matching_failed"}).bind(pause).execute(&self.0).await.map_err(err)?;
                Ok(true)
            }
        }
    }
    async fn matching_batch(
        &self,
        c: i64,
        account: i64,
        actor: i64,
        revision: i64,
    ) -> Result<bool> {
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query("SET LOCAL statement_timeout='20s'")
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let a = actor.to_string();
        let company = c.to_string();
        crate::draft::authorize(&mut tx, &a, &company, true).await?;
        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
            .bind(c)
            .fetch_one(&mut *tx)
            .await
            .map_err(err)?;
        if !locked {
            return Ok(false);
        }
        let config=sqlx::query("SELECT matching_cursor,matching_treasury FROM bank_connection WHERE company_id=$1 AND account_id=$2 AND revision=$3 AND matching_enabled AND matching_actor_id=$4 AND matching_next_at<=now() FOR UPDATE").bind(c).bind(account).bind(revision).bind(actor).fetch_optional(&mut *tx).await.map_err(err)?;
        let Some(config) = config else {
            return Ok(false);
        };
        let cursor: i64 = config.get("matching_cursor");
        let include_treasury: bool = config.get("matching_treasury");
        let ac = get_account(&mut tx, c, &account.to_string()).await?;
        if ac.archived {
            return Err(Error::Forbidden);
        }
        let mut ids = matching_ids(&mut tx, c, account, cursor).await?;
        if ids.is_empty() && cursor > 0 {
            ids = matching_ids(&mut tx, c, account, 0).await?;
        }
        let next_cursor = if ids.len() == 25 {
            *ids.last().ok_or(Error::Unavailable)?
        } else {
            0
        };
        let (checked, matched, items) = if ids.is_empty() {
            (0, 0, serde_json::json!([]))
        } else {
            let report = matching::batch_preview_scoped(
                &mut tx,
                c,
                BatchInput {
                    movement_ids: ids.iter().map(ToString::to_string).collect(),
                },
                include_treasury,
            )
            .await?;
            let items=serde_json::Value::Array(report.items.iter().map(|i|serde_json::json!({"movement_id":i.movement.id,"status":i.status,"target_kind":i.candidate.as_ref().map(|c|&c.target_kind),"document_id":i.candidate.as_ref().map(|c|&c.document_id)})).collect());
            let checked = report.items.len() as i32;
            let matched = matching::apply_batch(&mut tx, &a, &company, actor, c, report)
                .await?
                .len() as i32;
            (checked, matched, items)
        };
        sqlx::query("INSERT INTO bank_matching_run(company_id,account_id,actor_id,configuration_revision,checked,matched,items) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(c).bind(account).bind(actor).bind(revision).bind(checked).bind(matched).bind(items).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE bank_connection SET matching_cursor=$3,matching_next_at=now()+make_interval(secs=>$4),matching_error=NULL WHERE company_id=$1 AND account_id=$2").bind(c).bind(account).bind(next_cursor).bind(if next_cursor==0{300.0_f64}else{30.0_f64}).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(true)
    }
}
async fn matching_ids(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    account: i64,
    cursor: i64,
) -> Result<Vec<i64>> {
    // A manually detached match is an explicit user decision; never silently recreate it.
    sqlx::query_scalar("SELECT m.id FROM bank_movement m WHERE m.company_id=$1 AND m.account_id=$2 AND m.id>$3 AND NOT EXISTS(SELECT 1 FROM bank_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id) ORDER BY m.id LIMIT 25").bind(c).bind(account).bind(cursor).fetch_all(&mut **tx).await.map_err(err)
}

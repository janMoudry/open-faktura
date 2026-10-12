use async_trait::async_trait;
use faktura_application::import::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Row as SqlRow, Transaction};
use std::collections::{BTreeMap, HashSet};
#[derive(Clone)]
pub struct PgImport(pub PgPool);
fn fail(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn conflict(e: sqlx::Error) -> Error {
    if e.as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        Error::Conflict
    } else {
        fail(e)
    }
}
fn uuid(id: &str) -> Result<uuid::Uuid> {
    uuid::Uuid::parse_str(id).map_err(|_| Error::NotFound)
}
async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    write: bool,
) -> Result<(i64, i64)> {
    let actor = actor.parse::<i64>().map_err(|_| Error::NotFound)?;
    let company = company.parse::<i64>().map_err(|_| Error::NotFound)?;
    // Same lock order as issuing/settings; prevents races with numbering and contact writes.
    let query = if write {
        "SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR UPDATE OF c,m"
    } else {
        "SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR SHARE OF c,m"
    };
    let role: String = sqlx::query_scalar(query)
        .bind(company)
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await
        .map_err(fail)?
        .ok_or(Error::NotFound)?;
    if write && !["owner", "editor"].contains(&role.as_str()) {
        return Err(Error::Forbidden);
    }
    Ok((actor, company))
}
async fn check(
    tx: &mut Transaction<'_, Postgres>,
    company: i64,
    source: &str,
    agenda: &str,
    record: &Record,
) -> Result<(String, Vec<String>, Option<i64>)> {
    if let Some(row)=sqlx::query("SELECT data,COALESCE(contact_id,invoice_id) AS target FROM import_identity WHERE company_id=$1 AND source=$2 AND agenda=$3 AND kind=$4 AND source_key=$5").bind(company).bind(source).bind(agenda).bind(record.kind()).bind(record.key()).fetch_optional(&mut **tx).await.map_err(fail)? {
  return Ok(if row.get::<Value,_>("data")==serde_json::to_value(record).map_err(fail)? {("duplicate".into(),vec![],Some(row.get("target")))} else {("conflict".into(),vec!["changed_source".into()],None)});
 }
    let collision:bool=match record {
 Record::Contact{data,..}=>sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contact WHERE company_id=$1 AND (($2<>'' AND data->>'registration_number'=$2) OR ($3<>'' AND lower(data->>'email')=lower($3)) OR lower(data->>'name')=lower($4)))").bind(company).bind(&data.registration_number).bind(&data.email).bind(&data.name).fetch_one(&mut **tx).await.map_err(fail)?,
 Record::Invoice{number,payments,..}=>{
 let future:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM company WHERE id=$1 AND $2::text::date > (now() AT TIME ZONE (settings->>'timezone'))::date)").bind(company).bind(payments.iter().map(|p|p.paid_on.to_string()).max()).fetch_one(&mut **tx).await.map_err(fail)?;
 if future{return Ok(("invalid".into(),vec!["payment_date".into()],None));}
 sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invoice_draft WHERE company_id=$1 AND number=$2)").bind(company).bind(number).fetch_one(&mut **tx).await.map_err(fail)?
 }};
    Ok(if collision {
        ("conflict".into(), vec!["existing_target".into()], None)
    } else {
        ("ready".into(), vec![], None)
    })
}
async fn report(tx: &mut Transaction<'_, Postgres>, company: i64, id: uuid::Uuid) -> Result<Batch> {
    let batch =
        sqlx::query("SELECT source,agenda,status FROM import_batch WHERE company_id=$1 AND id=$2")
            .bind(company)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(fail)?
            .ok_or(Error::NotFound)?;
    let stored = sqlx::query(
        "SELECT * FROM import_row WHERE company_id=$1 AND batch_id=$2 ORDER BY position",
    )
    .bind(company)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(fail)?;
    let mut totals: BTreeMap<String, [Decimal; 4]> = BTreeMap::new();
    let mut rows = vec![];
    for r in stored {
        let status: String = r.get("status");
        let record: Record = serde_json::from_value(r.get("data")).map_err(fail)?;
        if let Record::Invoice {
            totals: t,
            payments,
            ..
        } = record
        {
            let values = totals.entry(t.currency).or_default();
            if let Some(n) = faktura_domain::import::money(&t.payable) {
                values[0] += n;
                if status == "imported" {
                    values[1] += n;
                }
                if ["skipped", "duplicate"].contains(&status.as_str()) {
                    values[2] += n;
                }
            }
            if status == "imported" {
                for p in payments {
                    if let Some(n) = faktura_domain::import::money(&p.amount) {
                        values[3] += n;
                    }
                }
            }
        }
        rows.push(Row {
            index: r.get("position"),
            kind: r.get("kind"),
            key: r.get("source_key"),
            label: r.get("label"),
            status,
            fields: serde_json::from_value(r.get("fields")).map_err(fail)?,
            target_id: r.get::<Option<i64>, _>("target_id").map(|v| v.to_string()),
        });
    }
    let money = |mut d: Decimal| {
        d.rescale(2);
        d.to_string()
    };
    Ok(Batch {
        id: id.to_string(),
        source: batch.get("source"),
        agenda: batch.get("agenda"),
        status: batch.get("status"),
        rows,
        limitations: vec![
            "unverified_source_presets".into(),
            "domestic_czk_only".into(),
            "original_pdf_and_attachments_not_transferred".into(),
            "number_series_unchanged".into(),
            "no_automatic_rollback".into(),
        ],
        totals: totals
            .into_iter()
            .map(|(currency, t)| CurrencyTotal {
                currency,
                source: money(t[0]),
                imported: money(t[1]),
                skipped: money(t[2]),
                payments: money(t[3]),
            })
            .collect(),
    })
}
#[async_trait]
impl ImportStore for PgImport {
    async fn prepare(&self, actor: &str, company: &str, input: Prepare) -> Result<Batch> {
        let mut tx = self.0.begin().await.map_err(fail)?;
        let (actor, company) = lock(&mut tx, actor, company, true).await?;
        let id = uuid(&input.request_id)?;
        let raw = serde_json::to_value(&input).map_err(fail)?;
        if raw.to_string().len() > 2_000_000 {
            return Err(Error::Invalid(vec!["size".into()]));
        }
        if let Some(existing) =
            sqlx::query("SELECT company_id,actor_id,input FROM import_batch WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(fail)?
        {
            if existing.get::<i64, _>("company_id") != company
                || existing.get::<i64, _>("actor_id") != actor
                || existing.get::<Value, _>("input") != raw
            {
                return Err(Error::Conflict);
            }
            return report(&mut tx, company, id).await;
        }
        sqlx::query("INSERT INTO import_batch(id,company_id,actor_id,source,agenda,input) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(company).bind(actor).bind(&input.source).bind(&input.agenda).bind(raw).execute(&mut *tx).await.map_err(conflict)?;
        let mut keys = HashSet::new();
        let mut duplicates = HashSet::new();
        for record in &input.records {
            let key = (record.kind().to_owned(), record.key().to_owned());
            if !keys.insert(key.clone()) {
                duplicates.insert(key);
            }
        }
        for (i, mut record) in input.records.into_iter().enumerate() {
            let mut errors = record.validate();
            if duplicates.contains(&(record.kind().into(), record.key().into())) {
                errors.push("duplicate_key".into());
            }
            let (status, fields, target) = if errors.is_empty() {
                check(&mut tx, company, &input.source, &input.agenda, &record).await?
            } else {
                ("invalid".into(), errors, None)
            };
            sqlx::query("INSERT INTO import_row(company_id,batch_id,position,kind,source_key,label,data,status,fields,target_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)").bind(company).bind(id).bind(i as i32).bind(record.kind()).bind(record.key()).bind(record.label()).bind(serde_json::to_value(&record).map_err(fail)?).bind(status).bind(json!(fields)).bind(target).execute(&mut *tx).await.map_err(fail)?;
        }
        // Ambiguities inside a single file are conflicts too, not silently merged.
        sqlx::query("UPDATE import_row r SET status='conflict',fields='[\"existing_target\"]'::jsonb WHERE r.batch_id=$1 AND r.status='ready' AND EXISTS(SELECT 1 FROM import_row c WHERE c.batch_id=r.batch_id AND c.position<>r.position AND c.kind=r.kind AND c.status IN ('ready','conflict') AND ((r.kind='invoice' AND c.data->>'number'=r.data->>'number') OR (r.kind='contact' AND (lower(c.data#>>'{data,name}')=lower(r.data#>>'{data,name}') OR (r.data#>>'{data,registration_number}'<>'' AND c.data#>>'{data,registration_number}'=r.data#>>'{data,registration_number}') OR (r.data#>>'{data,email}'<>'' AND lower(c.data#>>'{data,email}')=lower(r.data#>>'{data,email}'))))))").bind(id).execute(&mut *tx).await.map_err(fail)?;
        // Missing references are visible before confirmation, including contacts invalid in this batch.
        sqlx::query("UPDATE import_row r SET status='invalid',fields='[\"contact_key\"]'::jsonb WHERE r.batch_id=$1 AND r.status='ready' AND r.kind='invoice' AND r.data->>'contact_key' IS NOT NULL AND NOT EXISTS(SELECT 1 FROM import_identity i WHERE i.company_id=$2 AND i.source=$3 AND i.agenda=$4 AND i.kind='contact' AND i.source_key=r.data->>'contact_key') AND NOT EXISTS(SELECT 1 FROM import_row c WHERE c.batch_id=$1 AND c.kind='contact' AND c.source_key=r.data->>'contact_key' AND c.status IN ('ready','duplicate'))").bind(id).bind(company).bind(&input.source).bind(&input.agenda).execute(&mut *tx).await.map_err(fail)?;
        let value = report(&mut tx, company, id).await?;
        tx.commit().await.map_err(fail)?;
        Ok(value)
    }
    async fn get(&self, actor: &str, company: &str, id: &str) -> Result<Batch> {
        let mut tx = self.0.begin().await.map_err(fail)?;
        let (_, company) = lock(&mut tx, actor, company, false).await?;
        report(&mut tx, company, uuid(id)?).await
    }
    async fn list(&self, actor: &str, company: &str) -> Result<Vec<Batch>> {
        let mut tx = self.0.begin().await.map_err(fail)?;
        let (_, company) = lock(&mut tx, actor, company, false).await?;
        let ids: Vec<uuid::Uuid> = sqlx::query_scalar(
            "SELECT id FROM import_batch WHERE company_id=$1 ORDER BY created_at DESC LIMIT 20",
        )
        .bind(company)
        .fetch_all(&mut *tx)
        .await
        .map_err(fail)?;
        let mut result = vec![];
        for id in ids {
            result.push(report(&mut tx, company, id).await?);
        }
        Ok(result)
    }
    async fn apply(&self, actor: &str, company: &str, id: &str, input: Apply) -> Result<Batch> {
        let mut tx = self.0.begin().await.map_err(fail)?;
        let (actor, company) = lock(&mut tx, actor, company, true).await?;
        let id = uuid(id)?;
        let b = report(&mut tx, company, id).await?;
        if input
            .skip
            .iter()
            .any(|i| !b.rows.iter().any(|r| r.index == *i))
        {
            return Err(Error::Invalid(vec!["skip".into()]));
        }
        if b.status == "done" {
            return Ok(b);
        }
        for position in input.skip {
            sqlx::query("UPDATE import_row SET status='skipped' WHERE batch_id=$1 AND position=$2 AND status IN ('ready','invalid','conflict')").bind(id).bind(position).execute(&mut *tx).await.map_err(fail)?;
        }
        let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM import_row WHERE batch_id=$1 AND status IN ('invalid','conflict'))").bind(id).fetch_one(&mut *tx).await.map_err(fail)?;
        if !blocked {
            let rows=sqlx::query("SELECT position,data FROM import_row WHERE batch_id=$1 AND status='ready' ORDER BY kind,position LIMIT 25").bind(id).fetch_all(&mut *tx).await.map_err(fail)?;
            for row in rows {
                let position: i32 = row.get("position");
                let record: Record = serde_json::from_value(row.get("data")).map_err(fail)?;
                let (mut status, mut fields, mut target) =
                    check(&mut tx, company, &b.source, &b.agenda, &record).await?;
                if status == "ready" {
                    let contact = if let Record::Invoice {
                        contact_key: Some(key),
                        ..
                    } = &record
                    {
                        sqlx::query_scalar::<_,i64>("SELECT contact_id FROM import_identity WHERE company_id=$1 AND source=$2 AND agenda=$3 AND kind='contact' AND source_key=$4").bind(company).bind(&b.source).bind(&b.agenda).bind(key).fetch_optional(&mut *tx).await.map_err(fail)?
                    } else {
                        None
                    };
                    if matches!(
                        &record,
                        Record::Invoice {
                            contact_key: Some(_),
                            ..
                        }
                    ) && contact.is_none()
                    {
                        status = "conflict".into();
                        fields = vec!["contact_key".into()];
                    } else {
                        let inserted = insert(&mut tx, actor, company, &record, contact).await?;
                        target = Some(inserted);
                        status = "imported".into();
                        sqlx::query("INSERT INTO import_identity(company_id,source,agenda,kind,source_key,data,contact_id,invoice_id,batch_id,position) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)").bind(company).bind(&b.source).bind(&b.agenda).bind(record.kind()).bind(record.key()).bind(serde_json::to_value(&record).map_err(fail)?).bind(if record.kind()=="contact"{target}else{None}).bind(if record.kind()=="invoice"{target}else{None}).bind(id).bind(position).execute(&mut *tx).await.map_err(conflict)?;
                    }
                }
                sqlx::query("UPDATE import_row SET status=$3,fields=$4,target_id=$5 WHERE batch_id=$1 AND position=$2").bind(id).bind(position).bind(status).bind(json!(fields)).bind(target).execute(&mut *tx).await.map_err(fail)?;
            }
        }
        sqlx::query("UPDATE import_batch SET status=CASE WHEN EXISTS(SELECT 1 FROM import_row WHERE batch_id=$1 AND status IN ('ready','invalid','conflict')) THEN 'running' ELSE 'done' END,updated_at=now() WHERE id=$1").bind(id).execute(&mut *tx).await.map_err(fail)?;
        let value = report(&mut tx, company, id).await?;
        tx.commit().await.map_err(fail)?;
        Ok(value)
    }
}
async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    company: i64,
    record: &Record,
    contact: Option<i64>,
) -> Result<i64> {
    let request = uuid::Uuid::new_v4();
    match record {
        Record::Contact { data, .. } => {
            let data = serde_json::to_value(data).map_err(fail)?;
            let id:i64=sqlx::query_scalar("INSERT INTO contact(company_id,creator_id,request_id,initial_data,data) VALUES($1,$2,$3,$4,$4) RETURNING id").bind(company).bind(actor).bind(request).bind(&data).fetch_one(&mut **tx).await.map_err(conflict)?;
            sqlx::query("INSERT INTO contact_audit(company_id,contact_id,revision,actor_id,action,data,archived) VALUES($1,$2,1,$3,'create',$4,false)").bind(company).bind(id).bind(actor).bind(data).execute(&mut **tx).await.map_err(fail)?;
            Ok(id)
        }
        Record::Invoice {
            number,
            data,
            totals,
            payments,
            ..
        } => {
            let mut data = *data.clone();
            data.contact_id = contact.map(|v| v.to_string());
            let data = serde_json::to_value(data).map_err(fail)?;
            let totals = serde_json::to_value(totals).map_err(fail)?;
            let id:i64=sqlx::query_scalar("INSERT INTO invoice_draft(company_id,contact_id,creator_id,request_id,initial_data,data,totals,number,issued_at,issue_request_id,issue_actor_id,issue_revision) VALUES($1,$2,$3,$4,$5,$5,$6,$7,now(),$4,$3,1) RETURNING id").bind(company).bind(contact).bind(actor).bind(request).bind(&data).bind(&totals).bind(number).fetch_one(&mut **tx).await.map_err(conflict)?;
            sqlx::query("INSERT INTO invoice_draft_audit(company_id,draft_id,revision,actor_id,action,data,totals,number) VALUES($1,$2,1,$3,'issue',$4,$5,$6)").bind(company).bind(id).bind(actor).bind(data).bind(totals).bind(number).execute(&mut **tx).await.map_err(fail)?;
            for p in payments {
                sqlx::query("INSERT INTO invoice_payment(company_id,invoice_id,amount,paid_on,method,note,actor_id) VALUES($1,$2,$3::text::numeric,$4::text::date,$5,$6,$7)").bind(company).bind(id).bind(&p.amount).bind(p.paid_on.to_string()).bind(&p.method).bind(&p.note).bind(actor).execute(&mut **tx).await.map_err(fail)?;
            }
            Ok(id)
        }
    }
}

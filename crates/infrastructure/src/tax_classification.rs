use async_trait::async_trait;
use chrono::Datelike;
use faktura_application::{
    company::{CompanyError as Error, Result},
    tax_classification::*,
};
use faktura_domain::received::{amount, money};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
pub struct PgTaxClassification(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn source_sql() -> String {
    format!(
        r#"WITH tax_sources AS (WITH cash AS ({cash}), {tax} SELECT * FROM docs),
 turnover_sources AS ({period} SELECT id,kind,number,issued_on reported_on,amount,side FROM docs WHERE side='sales'
 UNION ALL SELECT id,kind,number,reported_on,net,side FROM ({cash}) cash WHERE side='sales' AND reported_on BETWEEN $2 AND $3),
 observed AS (SELECT coalesce(t.id,n.id) id,coalesce(t.kind,n.kind) kind,coalesce(t.number,n.number) number,coalesce(t.side,n.side) side,t.reported_on vat_on,n.reported_on turnover_on,t.amount recorded_vat_czk,n.amount turnover_net_czk FROM tax_sources t FULL JOIN turnover_sources n ON t.id=n.id AND t.kind=n.kind),
 fingerprinted AS (SELECT o.*,encode(sha256(convert_to(jsonb_build_array(o,
 CASE kind
 WHEN 'invoice' THEN (SELECT jsonb_build_array(data,totals,advances,number) FROM invoice_draft d WHERE d.company_id=$1 AND d.id=o.id)
 WHEN 'credit_note' THEN (SELECT jsonb_build_array(source,data,totals,status) FROM credit_note d WHERE d.company_id=$1 AND d.id=o.id)
 WHEN 'tax_receipt' THEN (SELECT snapshot FROM tax_receipt d WHERE d.company_id=$1 AND d.id=o.id)
 WHEN 'retail' THEN (SELECT jsonb_build_array(data,totals,cancelled) FROM retail_document d WHERE d.company_id=$1 AND d.id=o.id)
 WHEN 'received' THEN (SELECT jsonb_build_array(data,total,cancelled) FROM received_invoice d WHERE d.company_id=$1 AND d.id=o.id)
 WHEN 'purchase_receipt' THEN (SELECT jsonb_build_array(data,total,cancelled) FROM received_invoice d WHERE d.company_id=$1 AND d.id=o.id)
 WHEN 'cash' THEN (SELECT jsonb_build_array(data,tax,cancelled) FROM cash_movement d WHERE d.company_id=$1 AND d.id=o.id)
 END)::text,'UTF8')),'hex') fingerprint FROM observed o) "#,
        cash = crate::tax_report::CASH,
        tax = crate::tax_report::TAX,
        period = crate::period_report::SOURCE
    )
}
fn observation(r: &sqlx::postgres::PgRow) -> Observation {
    Observation {
        kind: r.get("kind"),
        id: r.get::<i64, _>("id").to_string(),
        number: r.get("number"),
        side: r.get("side"),
        vat_on: r.get("vat_on"),
        turnover_on: r.get("turnover_on"),
        recorded_vat_czk: r.get::<Option<Decimal>, _>("recorded_vat_czk").map(money),
        turnover_net_czk: r.get::<Option<Decimal>, _>("turnover_net_czk").map(money),
        fingerprint: r.get("fingerprint"),
    }
}
async fn observe(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    kind: &str,
    id: &str,
) -> Result<Observation> {
    let sql = format!(
        "{} SELECT * FROM fingerprinted WHERE kind=$5 AND id=$6",
        source_sql()
    );
    let r = sqlx::query(&sql)
        .bind(c)
        .bind(chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap())
        .bind(chrono::NaiveDate::from_ymd_opt(9999, 12, 31).unwrap())
        .bind(true)
        .bind(kind)
        .bind(id.parse::<i64>().map_err(|_| Error::NotFound)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    Ok(observation(&r))
}
fn classification(r: sqlx::postgres::PgRow, current: &Observation) -> Result<Classification> {
    let observation: Observation = serde_json::from_value(r.get("observation")).map_err(err)?;
    Ok(Classification {
        current: observation.fingerprint == current.fingerprint,
        revision: r.get("revision"),
        observation,
        data: serde_json::from_value(r.get("data")).map_err(err)?,
    })
}
#[async_trait]
impl TaxClassificationStore for PgTaxClassification {
    async fn get(&self, a: &str, c: &str, kind: &str, id: &str) -> Result<Source> {
        validate(kind, id, None)?;
        let mut tx = self
            .0
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ")
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let observed = observe(&mut tx, c, kind, id).await?;
        let record=sqlx::query("SELECT revision,observation,data FROM tax_evidence_classification WHERE company_id=$1 AND kind=$2 AND source_id=$3").bind(c).bind(kind).bind(id.parse::<i64>().unwrap()).fetch_optional(&mut *tx).await.map_err(err)?;
        let classification = record.map(|r| classification(r, &observed)).transpose()?;
        tx.commit().await.map_err(err)?;
        Ok(Source {
            observation: observed,
            classification,
        })
    }
    async fn set(
        &self,
        a: &str,
        c: &str,
        kind: &str,
        id: &str,
        input: Command,
    ) -> Result<Classification> {
        validate(kind, id, Some(&input))?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (actor, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let value = serde_json::json!({"kind":kind,"id":id,"command":input});
        if let Some(r)=sqlx::query("SELECT actor_id,input,result FROM tax_evidence_classification_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&input.request_id).fetch_optional(&mut *tx).await.map_err(err)? {if r.get::<i64,_>("actor_id")!=actor||r.get::<serde_json::Value,_>("input")!=value{return Err(Error::Conflict);}let mut result:Classification=serde_json::from_value(r.get("result")).map_err(err)?;
            result.current=match observe(&mut tx,c,kind,id).await{Ok(current)=>current.fingerprint==result.observation.fingerprint,Err(Error::NotFound)=>false,Err(e)=>return Err(e)};
            return Ok(result);}
        let observed = observe(&mut tx, c, kind, id).await?;
        if observed.fingerprint != input.fingerprint {
            return Err(Error::Conflict);
        }
        let today: chrono::NaiveDate = sqlx::query_scalar(
            "SELECT (now() AT TIME ZONE (settings->>'timezone'))::date FROM company WHERE id=$1",
        )
        .bind(c)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if input.data.turnover_included.is_some() && observed.turnover_net_czk.is_none() {
            return Err(Error::Invalid(vec!["turnover".into()]));
        }
        if let Some(claim) = &input.data.claimed_input_vat_czk {
            let claim = amount(claim)
                .filter(|v| *v >= Decimal::ZERO)
                .ok_or_else(|| Error::Invalid(vec!["claim".into()]))?;
            let recorded = observed
                .recorded_vat_czk
                .as_deref()
                .and_then(amount)
                .ok_or_else(|| Error::Invalid(vec!["claim".into()]))?;
            if observed.side != "purchases"
                || claim > recorded
                || input.data.claim_on.is_none_or(|d| {
                    d > today || d.year() < 1900 || observed.vat_on.is_some_and(|source| d < source)
                })
            {
                return Err(Error::Invalid(vec!["claim".into()]));
            }
        } else if input.data.claim_on.is_some() {
            return Err(Error::Invalid(vec!["claim".into()]));
        }
        let old:Option<i64>=sqlx::query_scalar("SELECT revision FROM tax_evidence_classification WHERE company_id=$1 AND kind=$2 AND source_id=$3 FOR UPDATE").bind(c).bind(kind).bind(id.parse::<i64>().unwrap()).fetch_optional(&mut *tx).await.map_err(err)?;
        if old.unwrap_or(0) != input.revision {
            return Err(Error::Conflict);
        }
        let revision = input.revision + 1;
        let observed_json = serde_json::to_value(&observed).map_err(err)?;
        let data = serde_json::to_value(&input.data).map_err(err)?;
        sqlx::query("INSERT INTO tax_evidence_classification(company_id,kind,source_id,observation,data,actor_id) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(company_id,kind,source_id) DO UPDATE SET revision=tax_evidence_classification.revision+1,observation=excluded.observation,data=excluded.data,actor_id=excluded.actor_id,updated_at=now()").bind(c).bind(kind).bind(id.parse::<i64>().unwrap()).bind(&observed_json).bind(&data).bind(actor).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("INSERT INTO tax_evidence_classification_audit(company_id,kind,source_id,revision,actor_id,observation,data) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(c).bind(kind).bind(id.parse::<i64>().unwrap()).bind(revision).bind(actor).bind(observed_json).bind(data).execute(&mut *tx).await.map_err(err)?;
        let result = Classification {
            revision,
            observation: observed,
            data: input.data.clone(),
            current: true,
        };
        sqlx::query("INSERT INTO tax_evidence_classification_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(input.request_id).bind(actor).bind(value).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn summary(&self, a: &str, c: &str, year: i32) -> Result<Summary> {
        let mut tx = self
            .0
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ")
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let today: chrono::NaiveDate = sqlx::query_scalar(
            "SELECT (now() AT TIME ZONE (settings->>'timezone'))::date FROM company WHERE id=$1",
        )
        .bind(c)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        if !(1900..=today.year()).contains(&year) {
            return Err(Error::Invalid(vec!["year".into()]));
        }
        // Full source range keeps fingerprints identical to individual lookups. Filter reporting dates afterwards.
        let sql=format!("{} SELECT f.*,v.observation old_observation,v.data classification_data FROM fingerprinted f LEFT JOIN tax_evidence_classification v ON v.company_id=$1 AND v.kind=f.kind AND v.source_id=f.id",source_sql());
        let rows = sqlx::query(&sql)
            .bind(c)
            .bind(chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap())
            .bind(chrono::NaiveDate::from_ymd_opt(9999, 12, 31).unwrap())
            .bind(true)
            .fetch_all(&mut *tx)
            .await
            .map_err(err)?;
        let mut included = Decimal::ZERO;
        let mut claimed = Decimal::ZERO;
        let (mut pending_turnover, mut pending_input, mut outdated, mut classified) = (0, 0, 0, 0);
        let within =
            |d: Option<chrono::NaiveDate>| d.is_some_and(|d| d.year() == year && d <= today);
        let mut keys = std::collections::HashSet::new();
        for r in rows {
            let o = observation(&r);
            keys.insert((o.kind.clone(), o.id.clone()));
            let old: Option<serde_json::Value> = r.get("old_observation");
            let data: Option<serde_json::Value> = r.get("classification_data");
            let old = old
                .map(serde_json::from_value::<Observation>)
                .transpose()
                .map_err(err)?;
            let data = data
                .map(serde_json::from_value::<Data>)
                .transpose()
                .map_err(err)?;
            let current = old
                .as_ref()
                .is_some_and(|old| old.fingerprint == o.fingerprint);
            if (within(o.vat_on)
                || within(o.turnover_on)
                || old
                    .as_ref()
                    .is_some_and(|old| within(old.vat_on) || within(old.turnover_on))
                || data.as_ref().is_some_and(|d| within(d.claim_on)))
                && old.is_some()
                && !current
            {
                outdated += 1;
            }
            if current
                && (within(o.vat_on)
                    || within(o.turnover_on)
                    || data.as_ref().is_some_and(|d| within(d.claim_on)))
            {
                classified += 1;
            }
            if within(o.turnover_on) && o.turnover_net_czk.is_some() {
                match data
                    .as_ref()
                    .filter(|_| current)
                    .and_then(|d| d.turnover_included)
                {
                    Some(true) => {
                        included += amount(o.turnover_net_czk.as_deref().unwrap())
                            .ok_or(Error::Unavailable)?
                    }
                    Some(false) => {}
                    None => pending_turnover += 1,
                }
            }
            if o.side == "purchases" && o.recorded_vat_czk.is_some() {
                if let Some(data) = data.as_ref().filter(|_| current) {
                    if let Some(value) = data.claimed_input_vat_czk.as_ref() {
                        if within(data.claim_on) {
                            claimed += amount(value).ok_or(Error::Unavailable)?;
                        }
                    } else if within(o.vat_on) {
                        pending_input += 1;
                    }
                } else if within(o.vat_on) {
                    pending_input += 1;
                }
            }
        }
        let historical = sqlx::query(
            "SELECT observation,data FROM tax_evidence_classification WHERE company_id=$1",
        )
        .bind(c)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
        for r in historical {
            let old: Observation = serde_json::from_value(r.get("observation")).map_err(err)?;
            let data: Data = serde_json::from_value(r.get("data")).map_err(err)?;
            if !keys.contains(&(old.kind.clone(), old.id.clone()))
                && (within(old.vat_on) || within(old.turnover_on) || within(data.claim_on))
            {
                outdated += 1;
            }
        }
        tx.commit().await.map_err(err)?;
        Ok(Summary {
            year,
            included_turnover_czk: money(included),
            claimed_input_vat_czk: money(claimed),
            unclassified_turnover: pending_turnover,
            unclassified_input: pending_input,
            outdated,
            classified,
        })
    }
}

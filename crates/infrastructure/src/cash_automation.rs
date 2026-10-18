//! Transaction-local cash projection. Never creates a payment itself.
use faktura_application::company::{CompanyError as Error, Result};
use faktura_domain::{
    cash::MovementData,
    payment::PaymentInput,
    received::{amount, money},
};
use sqlx::{Postgres, Row, Transaction};
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
pub(crate) struct Source<'a> {
    pub kind: &'a str,
    pub document: i64,
    pub payment_id: &'a str,
    pub payment: &'a PaymentInput,
}
pub(crate) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    company: i64,
    source: Source<'_>,
) -> Result<()> {
    if source.payment.method != "cash" || amount(&source.payment.amount).is_none_or(|v| v.is_zero())
    {
        return Ok(());
    }
    // Upgrade fixtures and older schema have no opt-in configuration. API startup always migrates.
    let installed: bool = sqlx::query_scalar("SELECT to_regclass('cash_settings') IS NOT NULL")
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    if !installed {
        return Ok(());
    }
    let account: Option<i64> =
        sqlx::query_scalar("SELECT account_id FROM cash_settings WHERE company_id=$1")
            .bind(company)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .flatten();
    let Some(account) = account else {
        return Ok(());
    };
    let (table, column, partner, incoming) = match source.kind {
        "invoice" => ("invoice_draft", "invoice_id", "customer", true),
        "received" => ("received_invoice", "received_id", "supplier", false),
        _ => return Err(Error::Unavailable),
    };
    let linked:bool=sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM cash_match WHERE company_id=$1 AND {column}=$2 AND payment_id=$3 AND detached_at IS NULL)")).bind(company).bind(source.document).bind(source.payment_id).fetch_one(&mut **tx).await.map_err(err)?;
    if linked {
        return Ok(());
    }
    let row = sqlx::query(&format!(
        "SELECT number,data FROM {table} WHERE company_id=$1 AND id=$2"
    ))
    .bind(company)
    .bind(source.document)
    .fetch_one(&mut **tx)
    .await
    .map_err(err)?;
    let number: String = row.get("number");
    let json: serde_json::Value = row.get("data");
    let value = amount(&source.payment.amount).ok_or(Error::Unavailable)?;
    let data = MovementData {
        booked_on: source.payment.paid_on,
        amount: money(if incoming { value } else { -value }),
        currency: "CZK".into(),
        variable_symbol: if incoming {
            json["payment"]["variable_symbol"].as_str()
        } else {
            json["variable_symbol"].as_str()
        }
        .unwrap_or_default()
        .into(),
        counterparty: json[partner]["name"].as_str().unwrap_or_default().into(),
        description: number,
        person: String::new(),
    };
    let id = crate::cash::insert_movement(tx, company, &account.to_string(), &data)
        .await
        .map_err(|e| match e {
            Error::Invalid(_) => Error::Invalid(vec!["cash_automatic_date".into()]),
            other => other,
        })?;
    sqlx::query("UPDATE cash_movement SET automatic=true WHERE company_id=$1 AND id=$2")
        .bind(company)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    sqlx::query("INSERT INTO cash_match(company_id,movement_id,invoice_id,received_id,payment_id,created_payment,actor_id) VALUES($1,$2,$3,$4,$5,false,$6)").bind(company).bind(id).bind(if incoming{Some(source.document)}else{None}).bind(if incoming{None}else{Some(source.document)}).bind(source.payment_id).bind(actor).execute(&mut **tx).await.map_err(err)?;
    audit(tx, actor, company, id, "automatic_create").await
}
pub(crate) async fn reverse(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    company: i64,
    kind: &str,
    document: i64,
    payment_id: Option<&str>,
    reason: &str,
) -> Result<()> {
    let column = match kind {
        "invoice" => "invoice_id",
        "received" => "received_id",
        _ => return Err(Error::Unavailable),
    };
    let ids:Vec<i64>=sqlx::query_scalar(&format!("SELECT m.id FROM cash_movement m JOIN cash_match b ON b.company_id=m.company_id AND b.movement_id=m.id WHERE m.company_id=$1 AND b.{column}=$2 AND ($3::text IS NULL OR b.payment_id=$3) AND m.automatic AND NOT m.cancelled AND b.detached_at IS NULL FOR UPDATE OF m")).bind(company).bind(document).bind(payment_id).fetch_all(&mut **tx).await.map_err(err)?;
    for id in ids {
        sqlx::query("UPDATE cash_match SET detached_at=now(),detached_by=$3,reason=$4 WHERE company_id=$1 AND movement_id=$2 AND detached_at IS NULL").bind(company).bind(id).bind(actor).bind(reason).execute(&mut **tx).await.map_err(err)?;
        sqlx::query("UPDATE cash_movement SET cancelled=true,reason=$3,revision=revision+1 WHERE company_id=$1 AND id=$2").bind(company).bind(id).bind(reason).execute(&mut **tx).await.map_err(err)?;
        audit(tx, actor, company, id, "automatic_reverse").await?;
    }
    Ok(())
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: i64,
    company: i64,
    movement: i64,
    action: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO cash_command(company_id,request_id,actor_id,input,result) SELECT $1,$2::text::uuid,$3,$4,to_jsonb(m) FROM cash_movement m WHERE company_id=$1 AND id=$5").bind(company).bind(uuid::Uuid::new_v4().to_string()).bind(actor).bind(serde_json::json!({"action":action,"movement_id":movement.to_string()})).bind(movement).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}

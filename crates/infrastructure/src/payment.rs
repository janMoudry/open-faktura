use async_trait::async_trait;
use faktura_application::payment::*;
use sqlx::{PgPool, Postgres, Row, Transaction};
#[derive(Clone)]
pub struct PgPayment(pub PgPool);
fn unavailable(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    invoice: &str,
    write: bool,
) -> Result<(i64, i64, i64)> {
    let (actor, company) = crate::draft::authorize(tx, actor, company, write).await?;
    let invoice = id(invoice)?;
    let sql = if write {
        "SELECT number FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE"
    } else {
        "SELECT number FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR SHARE"
    };
    let row = sqlx::query(sql)
        .bind(company)
        .bind(invoice)
        .fetch_optional(&mut **tx)
        .await
        .map_err(unavailable)?
        .ok_or(Error::NotFound)?;
    if row.get::<Option<String>, _>("number").is_none() {
        return Err(Error::Conflict);
    }
    Ok((actor, company, invoice))
}
async fn load(tx: &mut Transaction<'_, Postgres>, company: i64, invoice: i64) -> Result<Payments> {
    let summary: serde_json::Value = sqlx::query_scalar(
        "SELECT settlement FROM invoice_with_settlement WHERE company_id=$1 AND id=$2",
    )
    .bind(company)
    .bind(invoice)
    .fetch_one(&mut **tx)
    .await
    .map_err(unavailable)?;
    let rows=sqlx::query("SELECT p.id::text,p.amount::text,p.paid_on::text,p.method,p.note,p.actor_id::text,p.created_at::text,r.created_at::text AS reversed_at,r.reason,r.actor_id::text AS reversed_by,r.replacement_id::text FROM invoice_payment p LEFT JOIN invoice_payment_reversal r ON r.company_id=p.company_id AND r.invoice_id=p.invoice_id AND r.payment_id=p.id WHERE p.company_id=$1 AND p.invoice_id=$2 ORDER BY p.paid_on DESC,p.id DESC")
 .bind(company).bind(invoice).fetch_all(&mut **tx).await.map_err(unavailable)?;
    Ok(Payments {
        summary: serde_json::from_value(summary).map_err(unavailable)?,
        items: rows
            .into_iter()
            .map(|r| Payment {
                id: r.get("id"),
                amount: r.get("amount"),
                paid_on: r.get("paid_on"),
                method: r.get("method"),
                note: r.get("note"),
                actor_id: r.get("actor_id"),
                created_at: r.get("created_at"),
                reversed_at: r.get("reversed_at"),
                reason: r.get("reason"),
                reversed_by: r.get("reversed_by"),
                replacement_id: r.get("replacement_id"),
            })
            .collect(),
    })
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    a: i64,
    i: i64,
    key: &str,
    input: &serde_json::Value,
) -> Result<Option<Payments>> {
    let row=sqlx::query("SELECT actor_id,invoice_id,input,result FROM invoice_payment_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(key).fetch_optional(&mut **tx).await.map_err(unavailable)?;
    if let Some(r) = row {
        if r.get::<i64, _>("actor_id") != a
            || r.get::<i64, _>("invoice_id") != i
            || r.get::<serde_json::Value, _>("input") != *input
        {
            return Err(Error::Conflict);
        }
        return Ok(Some(
            serde_json::from_value(r.get("result")).map_err(unavailable)?,
        ));
    }
    Ok(None)
}
async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    a: i64,
    i: i64,
    p: &PaymentInput,
) -> Result<i64> {
    let valid:bool=sqlx::query_scalar("SELECT $3::text::date BETWEEN (d.data->>'issued_on')::date AND s.today AND s.payable>0 FROM invoice_draft d JOIN invoice_settlement s ON s.company_id=d.company_id AND s.invoice_id=d.id WHERE d.company_id=$1 AND d.id=$2")
 .bind(c).bind(i).bind(p.paid_on.to_string()).fetch_one(&mut **tx).await.map_err(unavailable)?;
    if !valid {
        return Err(Error::Invalid(vec!["payment_date_or_zero_invoice".into()]));
    }
    sqlx::query_scalar("INSERT INTO invoice_payment(company_id,invoice_id,amount,paid_on,method,note,actor_id) VALUES($1,$2,$3::text::numeric,$4::text::date,$5,$6,$7) RETURNING id")
 .bind(c).bind(i).bind(&p.amount).bind(p.paid_on.to_string()).bind(&p.method).bind(&p.note).bind(a).fetch_one(&mut **tx).await.map_err(unavailable)
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    a: i64,
    i: i64,
    key: &str,
    input: serde_json::Value,
    value: &Payments,
) -> Result<()> {
    sqlx::query("INSERT INTO invoice_payment_command(company_id,invoice_id,actor_id,request_id,input,result) VALUES($1,$2,$3,$4::text::uuid,$5,$6)")
 .bind(c).bind(i).bind(a).bind(key).bind(input).bind(serde_json::to_value(value).map_err(unavailable)?).execute(&mut **tx).await.map_err(|e|if e.as_database_error().is_some_and(|e|e.is_unique_violation()){Error::Conflict}else{unavailable(e)})?;
    Ok(())
}
#[async_trait]
impl PaymentStore for PgPayment {
    async fn get(&self, a: &str, c: &str, i: &str) -> Result<Payments> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (_, c, i) = lock(&mut tx, a, c, i, false).await?;
        let v = load(&mut tx, c, i).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(v)
    }
    async fn register(&self, a: &str, c: &str, i: &str, cmd: Register) -> Result<Payments> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (a, c, i) = lock(&mut tx, a, c, i, true).await?;
        let input = serde_json::json!({"operation":"register","command":cmd});
        if let Some(v) = replay(&mut tx, c, a, i, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        if load(&mut tx, c, i).await?.summary.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        insert(&mut tx, c, a, i, &cmd.payment).await?;
        let v = load(&mut tx, c, i).await?;
        record(&mut tx, c, a, i, &cmd.request_id, input, &v).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(v)
    }
    async fn reverse(&self, a: &str, c: &str, i: &str, p: &str, cmd: Reverse) -> Result<Payments> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (a, c, i) = lock(&mut tx, a, c, i, true).await?;
        let p = id(p)?;
        let input =
            serde_json::json!({"operation":"reverse","payment_id":p.to_string(),"command":cmd});
        if let Some(v) = replay(&mut tx, c, a, i, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        let current = load(&mut tx, c, i).await?;
        let original = current
            .items
            .iter()
            .find(|v| v.id == p.to_string())
            .ok_or(Error::NotFound)?;
        if current.summary.revision != cmd.revision || original.reversed_at.is_some() {
            return Err(Error::Conflict);
        }
        let replacement = if let Some(p) = &cmd.replacement {
            Some(insert(&mut tx, c, a, i, p).await?)
        } else {
            None
        };
        sqlx::query("INSERT INTO invoice_payment_reversal(company_id,invoice_id,payment_id,reason,actor_id,replacement_id) VALUES($1,$2,$3,$4,$5,$6)")
  .bind(c).bind(i).bind(p).bind(cmd.reason.trim()).bind(a).bind(replacement).execute(&mut *tx).await.map_err(unavailable)?;
        let v = load(&mut tx, c, i).await?;
        record(&mut tx, c, a, i, &cmd.request_id, input, &v).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(v)
    }
    async fn overview(&self, a: &str, c: &str) -> Result<Overview> {
        let mut tx = self.0.begin().await.map_err(unavailable)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let r=sqlx::query("SELECT count(*) AS issued_count,count(*) FILTER(WHERE outstanding>0) AS outstanding_count,count(*) FILTER(WHERE overdue) AS overdue_count,COALESCE(sum(outstanding),0)::numeric(30,2)::text AS outstanding,COALESCE(sum(outstanding) FILTER(WHERE overdue),0)::numeric(30,2)::text AS overdue,COALESCE(sum(paid),0)::numeric(30,2)::text AS received,COALESCE(sum(overpayment),0)::numeric(30,2)::text AS overpayment,(SELECT count(*) FROM invoice_draft WHERE company_id=$1 AND number IS NULL) AS draft_count,(SELECT (now() AT TIME ZONE (settings->>'timezone'))::date::text FROM company WHERE id=$1) AS today FROM invoice_settlement WHERE company_id=$1")
  .bind(c).fetch_one(&mut *tx).await.map_err(unavailable)?;
        let v = Overview {
            issued_count: r.get("issued_count"),
            draft_count: r.get("draft_count"),
            outstanding_count: r.get("outstanding_count"),
            overdue_count: r.get("overdue_count"),
            outstanding: r.get("outstanding"),
            overdue: r.get("overdue"),
            received: r.get("received"),
            overpayment: r.get("overpayment"),
            currency: "CZK".into(),
            today: r.get("today"),
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(v)
    }
}

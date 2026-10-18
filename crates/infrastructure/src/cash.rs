#[path = "cash_matching.rs"]
mod matching;
use async_trait::async_trait;
use chrono::Datelike;
use faktura_application::{cash::*, payment, received};
use faktura_domain::{
    payment::PaymentInput,
    received::{amount, money, Kind},
};
use sqlx::{PgPool, Postgres, Row, Transaction};

#[derive(Clone)]
pub struct PgCash(pub PgPool);
fn err(e: impl std::fmt::Display) -> Error {
    let _ = e;
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn invalid(f: &str) -> Error {
    Error::Invalid(vec![f.into()])
}
fn account(r: sqlx::postgres::PgRow) -> Account {
    Account {
        id: r.get::<i64, _>("id").to_string(),
        revision: r.get("revision"),
        archived: r.get("archived"),
        data: AccountData {
            name: r.get("name"),
            opening_on: r.get("opening_on"),
            opening_balance: money(r.get("opening_balance")),
            currency: r.get("currency"),
        },
    }
}
async fn get_account(tx: &mut Transaction<'_, Postgres>, c: i64, a: &str) -> Result<Account> {
    Ok(account(
        sqlx::query("SELECT * FROM cash_account WHERE company_id=$1 AND id=$2 FOR SHARE")
            .bind(c)
            .bind(id(a)?)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?,
    ))
}
async fn movement(tx: &mut Transaction<'_, Postgres>, c: i64, m: i64) -> Result<Movement> {
    let r = sqlx::query("SELECT * FROM cash_movement WHERE company_id=$1 AND id=$2 FOR UPDATE")
        .bind(c)
        .bind(m)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    let matches=sqlx::query("SELECT b.*,coalesce(i.number,r.number,s.number,n.number) AS document_number,CASE WHEN to_jsonb(b)->>'advance_id' IS NOT NULL THEN 'advance' WHEN to_jsonb(b)->>'credit_id' IS NOT NULL THEN 'credit' ELSE coalesce(r.kind,'invoice') END AS document_kind FROM cash_match b LEFT JOIN invoice_draft i ON i.company_id=b.company_id AND i.id=b.invoice_id LEFT JOIN received_invoice r ON r.company_id=b.company_id AND r.id=b.received_id LEFT JOIN sales_document s ON s.company_id=b.company_id AND s.id=(to_jsonb(b)->>'advance_id')::bigint LEFT JOIN credit_note n ON n.company_id=b.company_id AND n.id=(to_jsonb(b)->>'credit_id')::bigint WHERE b.company_id=$1 AND b.movement_id=$2 ORDER BY b.id")
      .bind(c).bind(m).fetch_all(&mut **tx).await.map_err(err)?.into_iter().map(|r|{
        let invoice:Option<i64>=r.get("invoice_id");
        Match{id:r.get::<i64,_>("id").to_string(),target_kind:if invoice.is_some(){"invoice"}else if r.try_get::<Option<i64>,_>("advance_id").unwrap_or(None).is_some(){"advance"}else if r.try_get::<Option<i64>,_>("credit_id").unwrap_or(None).is_some(){"credit"}else{"received"}.into(),
            document_id:invoice.or(r.get("received_id")).or(r.try_get("advance_id").unwrap_or(None)).or(r.try_get("credit_id").unwrap_or(None)).expect("match target constraint").to_string(),document_number:r.get("document_number"),document_kind:r.get("document_kind"),
            payment_id:r.get("payment_id"),created_payment:r.get("created_payment"),created_at:r.get::<chrono::DateTime<chrono::Utc>,_>("created_at").to_rfc3339(),
            actor_id:r.get::<i64,_>("actor_id").to_string(),detached_at:r.get::<Option<chrono::DateTime<chrono::Utc>>,_>("detached_at").map(|v|v.to_rfc3339()),
            detached_by:r.get::<Option<i64>,_>("detached_by").map(|v|v.to_string()),reason:r.get("reason")}
      }).collect();
    Ok(Movement {
        retail_posting_id: r
            .try_get::<Option<i64>, _>("retail_posting_id")
            .unwrap_or(None)
            .map(|v| v.to_string()),
        id: m.to_string(),
        tax: r
            .get::<Option<serde_json::Value>, _>("tax")
            .map(serde_json::from_value)
            .transpose()
            .map_err(err)?,
        automatic: r.get("automatic"),
        account_id: r.get::<i64, _>("account_id").to_string(),
        number: r.get("number"),
        revision: r.get("revision"),
        cancelled: r.get("cancelled"),
        reason: r.get("reason"),
        data: serde_json::from_value(r.get("data")).map_err(err)?,
        matches,
    })
}
pub(crate) async fn insert_movement(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    account: &str,
    data: &MovementData,
) -> Result<i64> {
    let a = get_account(tx, c, account).await?;
    if a.archived {
        return Err(Error::Conflict);
    }
    let e = data.validate();
    if !e.is_empty() {
        return Err(Error::Invalid(e));
    }
    let valid: bool = sqlx::query_scalar("SELECT $1 <= (now() AT TIME ZONE 'Europe/Prague')::date")
        .bind(data.booked_on)
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    if !valid {
        return Err(invalid("booked_on"));
    }
    if data.booked_on < a.data.opening_on {
        return Err(invalid("booked_on"));
    }
    let year = data.booked_on.year();
    let incoming = amount(&data.amount)
        .ok_or_else(|| invalid("amount"))?
        .is_sign_positive();
    let direction = if incoming { "income" } else { "expense" };
    let next: Option<i64> = sqlx::query_scalar(
        "SELECT next_value FROM cash_series WHERE company_id=$1 AND year=$2 AND direction=$3",
    )
    .bind(c)
    .bind(year)
    .bind(direction)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?;
    if next.is_some_and(|v| v >= 1000000) {
        return Err(invalid("numbering_exhausted"));
    }
    let seq:i64=sqlx::query_scalar("INSERT INTO cash_series(company_id,year,direction,next_value) VALUES($1,$2,$3,2) ON CONFLICT(company_id,year,direction) DO UPDATE SET next_value=cash_series.next_value+1 RETURNING next_value-1").bind(c).bind(year).bind(direction).fetch_one(&mut **tx).await.map_err(err)?;
    sqlx::query_scalar("INSERT INTO cash_movement(company_id,account_id,number,data,amount,booked_on,issuer) VALUES($1,$2,$3,$4,$5,$6,(SELECT settings FROM company WHERE id=$1)) RETURNING id")
      .bind(c).bind(id(account)?).bind(format!("{}{year}{seq:06}",if incoming{"PP"}else{"VP"})).bind(serde_json::to_value(data).map_err(err)?).bind(amount(&data.amount).ok_or_else(||invalid("amount"))?).bind(data.booked_on).fetch_one(&mut **tx).await.map_err(err)
}
async fn linked(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    kind: &str,
    doc: i64,
    payment: &str,
) -> Result<bool> {
    let column = if kind == "invoice" {
        "invoice_id"
    } else {
        "received_id"
    };
    sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM cash_match WHERE company_id=$1 AND {column}=$2 AND payment_id=$3 AND detached_at IS NULL)")).bind(c).bind(doc).bind(payment).fetch_one(&mut **tx).await.map_err(err)
}
async fn candidates(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    m: &Movement,
    q: &str,
    only: Option<i64>,
) -> Result<Vec<Candidate>> {
    if q.len() > 200 {
        return Err(invalid("query"));
    }
    if m.retail_posting_id.is_some()
        || m.tax.is_some()
        || m.cancelled
        || m.matches.iter().any(|v| v.detached_at.is_none())
    {
        return Ok(vec![]);
    }
    let a = get_account(tx, c, &m.account_id).await?;
    if a.archived {
        return Ok(vec![]);
    }
    let value = amount(&m.data.amount).ok_or(Error::Unavailable)?;
    let incoming = value.is_sign_positive();
    let value = value.abs();
    let mut result = vec![];
    let table = if incoming {
        "invoice_draft"
    } else {
        "received_invoice"
    };
    let filter = if incoming {
        "number IS NOT NULL"
    } else {
        "NOT cancelled"
    };
    let partner = if incoming { "customer" } else { "supplier" };
    let rows=sqlx::query(&format!("SELECT id,number,data FROM {table} WHERE company_id=$1 AND ($3::bigint IS NULL OR id=$3) AND {filter} AND ($2='' OR position(lower($2) in lower(number||' '||coalesce(data->'{partner}'->>'name','')||' '||coalesce(data->>'external_number','')||' '||coalesce(data->'payment'->>'variable_symbol',data->>'variable_symbol','')))>0) ORDER BY id DESC LIMIT 50"))
        .bind(c).bind(q).bind(only).fetch_all(&mut **tx).await.map_err(err)?;
    for r in rows {
        let doc: i64 = r.get("id");
        let json: serde_json::Value = r.get("data");
        let number: String = r.get("number");
        let name = json[partner]["name"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let currency = if incoming {
            json["calculation"]["currency"].as_str()
        } else {
            json["currency"].as_str()
        };
        if currency != Some("CZK") {
            continue;
        }
        if incoming {
            let payments = crate::payment::load(tx, c, doc).await?;
            let eligible_date = json["issued_on"]
                .as_str()
                .is_some_and(|d| d <= m.data.booked_on.to_string().as_str());
            if eligible_date && amount(&payments.summary.outstanding).is_some_and(|v| v >= value) {
                result.push(Candidate {
                    available: money(value),
                    target_kind: "invoice".into(),
                    document_kind: "invoice".into(),
                    document_id: doc.to_string(),
                    number: number.clone(),
                    partner: name.clone(),
                    revision: payments.summary.revision,
                    payment_id: None,
                    outstanding: payments.summary.outstanding.clone(),
                });
            }
            for p in payments.items.iter().filter(|p| {
                p.reversed_at.is_none()
                    && p.method == "cash"
                    && p.paid_on == m.data.booked_on.to_string()
                    && amount(&p.amount) == Some(value)
            }) {
                if !linked(tx, c, "invoice", doc, &p.id).await? {
                    result.push(Candidate {
                        available: money(value),
                        target_kind: "invoice".into(),
                        document_kind: "invoice".into(),
                        document_id: doc.to_string(),
                        number: number.clone(),
                        partner: name.clone(),
                        revision: payments.summary.revision,
                        payment_id: Some(p.id.clone()),
                        outstanding: payments.summary.outstanding.clone(),
                    });
                }
            }
        } else {
            let d = crate::received::load(tx, c, doc).await?;
            if d.data.kind == Kind::Invoice
                && d.data.issued_on <= m.data.booked_on
                && amount(&d.remaining).is_some_and(|v| v >= value)
            {
                result.push(Candidate {
                    available: money(value),
                    target_kind: "received".into(),
                    document_kind: "invoice".into(),
                    document_id: doc.to_string(),
                    number: number.clone(),
                    partner: name.clone(),
                    revision: d.revision,
                    payment_id: None,
                    outstanding: d.remaining.clone(),
                });
            }
            for p in d.payments.iter().filter(|p| {
                p.reversed_at.is_none()
                    && p.input.method == "cash"
                    && p.input.paid_on == m.data.booked_on
                    && amount(&p.input.amount) == Some(value)
            }) {
                if !linked(tx, c, "received", doc, &p.id).await? {
                    result.push(Candidate {
                        available: money(value),
                        target_kind: "received".into(),
                        document_kind: d.data.kind.as_str().into(),
                        document_id: doc.to_string(),
                        number: number.clone(),
                        partner: name.clone(),
                        revision: d.revision,
                        payment_id: Some(p.id.clone()),
                        outstanding: d.remaining.clone(),
                    });
                }
            }
        }
    }
    if only.is_none() {
        result.extend(
            crate::treasury_documents::candidates(
                tx,
                c,
                incoming,
                value,
                m.data.booked_on,
                "cash",
                "CZK",
                q,
                None,
                false,
            )
            .await?,
        );
    }
    Ok(result)
}
#[async_trait]
impl CashStore for PgCash {
    async fn terminals(&self, a: &str, c: &str) -> Result<Vec<Terminal>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let rows = sqlx::query(
            "SELECT * FROM sales_terminal WHERE company_id=$1 ORDER BY archived,name,id",
        )
        .bind(c)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?;
        Ok(rows.into_iter().map(terminal).collect())
    }

    async fn tax_preview(&self, a: &str, c: &str, request: TaxRequest) -> Result<TaxSnapshot> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        tax_snapshot(&mut tx, c, &request).await
    }

    async fn settings(&self, a: &str, c: &str) -> Result<Settings> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        read_settings(&mut tx, c).await
    }
    async fn pdf(&self, a: &str, c: &str, m: &str, language: &str) -> Result<Vec<u8>> {
        crate::cash_pdf::load(&self.0, a, c, m, language).await
    }

    async fn accounts(&self, a: &str, c: &str) -> Result<Vec<Account>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        Ok(
            sqlx::query("SELECT * FROM cash_account WHERE company_id=$1 ORDER BY archived,name,id")
                .bind(c)
                .fetch_all(&mut *tx)
                .await
                .map_err(err)?
                .into_iter()
                .map(account)
                .collect(),
        )
    }
    async fn get(&self, a: &str, c: &str, m: &str) -> Result<Movement> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        movement(&mut tx, c, id(m)?).await
    }
    async fn candidates(&self, a: &str, c: &str, m: &str, q: &str) -> Result<Vec<Candidate>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let m = movement(&mut tx, c, id(m)?).await?;
        candidates(&mut tx, c, &m, q, None).await
    }
    async fn movements(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        if q.query.len() > 200
            || !(1..=10000).contains(&q.page.unwrap_or(1))
            || !["", "all", "income", "expense"].contains(&q.direction.as_str())
            || !["", "all", "matched", "unmatched", "cancelled", "tax"].contains(&q.status.as_str())
            || matches!((q.from,q.to),(Some(f),Some(t)) if f>t)
        {
            return Err(invalid("query"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let account_id = if q.account_id.is_empty() {
            None
        } else {
            Some(id(&q.account_id)?)
        };
        let predicate="company_id=$1 AND ($6::date IS NULL OR booked_on >= $6) AND ($7::date IS NULL OR booked_on <= $7) AND ($2::bigint IS NULL OR account_id=$2) AND CASE $3 WHEN 'income' THEN amount>0 WHEN 'expense' THEN amount<0 ELSE true END AND ($4='' OR position(lower($4) in lower(number||' '||(data->>'variable_symbol')||' '||(data->>'counterparty')||' '||(data->>'description')))>0) AND CASE $5 WHEN 'cancelled' THEN cancelled WHEN 'matched' THEN NOT cancelled AND EXISTS(SELECT 1 FROM cash_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL) WHEN 'tax' THEN NOT cancelled AND tax IS NOT NULL WHEN 'unmatched' THEN NOT cancelled AND tax IS NULL AND NOT EXISTS(SELECT 1 FROM cash_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL) ELSE true END";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM cash_movement m WHERE {predicate}"
        ))
        .bind(c)
        .bind(account_id)
        .bind(&q.direction)
        .bind(&q.query)
        .bind(&q.status)
        .bind(q.from)
        .bind(q.to)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let ids:Vec<i64>=sqlx::query_scalar(&format!("SELECT id FROM cash_movement m WHERE {predicate} ORDER BY booked_on DESC,id DESC LIMIT 25 OFFSET $8")).bind(c).bind(account_id).bind(q.direction).bind(q.query).bind(q.status).bind(q.from).bind(q.to).bind((q.page.unwrap_or(1)-1)*25).fetch_all(&mut *tx).await.map_err(err)?;
        let mut items = vec![];
        for m in ids {
            items.push(movement(&mut tx, c, m).await?);
        }
        Ok(Page {
            items,
            total,
            page: q.page.unwrap_or(1),
        })
    }
    async fn balance(&self, a: &str, company: &str, q: Search) -> Result<Balance> {
        if matches!((q.from,q.to),(Some(f),Some(t)) if f>t) {
            return Err(invalid("date"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, company, false).await?;
        let account = get_account(&mut tx, c, &q.account_id).await?;
        if q.from.is_some_and(|f| f < account.data.opening_on)
            || q.to.is_some_and(|t| t < account.data.opening_on)
        {
            return Err(invalid("date"));
        }
        let row=sqlx::query("SELECT coalesce(sum(amount) FILTER(WHERE booked_on < $3),0) AS before,coalesce(sum(amount) FILTER(WHERE ($3::date IS NULL OR booked_on >= $3) AND amount>0),0) AS income,coalesce(-sum(amount) FILTER(WHERE ($3::date IS NULL OR booked_on >= $3) AND amount<0),0) AS expense,count(*) FILTER(WHERE $3::date IS NULL OR booked_on >= $3) AS count FROM cash_movement WHERE company_id=$1 AND account_id=$2 AND NOT cancelled AND ($4::date IS NULL OR booked_on <= $4)")
          .bind(c).bind(id(&q.account_id)?).bind(q.from).bind(q.to).fetch_one(&mut *tx).await.map_err(err)?;
        let opening = amount(&account.data.opening_balance).ok_or(Error::Unavailable)?
            + row.get::<rust_decimal::Decimal, _>("before");
        let income: rust_decimal::Decimal = row.get("income");
        let expense: rust_decimal::Decimal = row.get("expense");
        Ok(Balance {
            opening: money(opening),
            income: money(income),
            expense: money(expense),
            closing: money(opening + income - expense),
            count: row.get("count"),
        })
    }
    async fn execute(&self, actor: &str, company: &str, cmd: Command) -> Result<Reply> {
        if uuid::Uuid::parse_str(&cmd.request_id).is_err() {
            return Err(invalid("request_id"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, actor, company, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r) = sqlx::query(
            "SELECT * FROM cash_command WHERE company_id=$1 AND request_id=$2::text::uuid",
        )
        .bind(c)
        .bind(&cmd.request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        {
            if r.get::<i64, _>("actor_id") != a || r.get::<serde_json::Value, _>("input") != input {
                return Err(Error::Conflict);
            }
            return serde_json::from_value(r.get("result")).map_err(err);
        }
        let result = match cmd.action {
            Action::Terminal {
                id: terminal_id,
                revision,
                code,
                name,
                archived,
            } => {
                let code = code.trim().to_ascii_uppercase();
                let name = name.trim();
                if code.is_empty()
                    || code.len() > 32
                    || !code.bytes().all(|b| {
                        b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
                    })
                    || name.is_empty()
                    || name.chars().count() > 100
                    || name.chars().any(char::is_control)
                {
                    return Err(invalid("terminal"));
                }
                let existing = terminal_id.as_deref().map(id).transpose()?;
                if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM sales_terminal WHERE company_id=$1 AND code=$2 AND ($3::bigint IS NULL OR id<>$3))").bind(c).bind(&code).bind(existing).fetch_one(&mut *tx).await.map_err(err)? {return Err(Error::Conflict);}
                let r = if let Some(i) = existing {
                    sqlx::query("UPDATE sales_terminal SET code=$3,name=$4,archived=$5,revision=revision+1 WHERE company_id=$1 AND id=$2 AND revision=$6 RETURNING *").bind(c).bind(i).bind(code).bind(name).bind(archived).bind(revision.ok_or_else(||invalid("revision"))?).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::Conflict)?
                } else {
                    if revision.is_some() {
                        return Err(invalid("revision"));
                    }
                    sqlx::query("INSERT INTO sales_terminal(company_id,code,name,archived) VALUES($1,$2,$3,$4) RETURNING *").bind(c).bind(code).bind(name).bind(archived).fetch_one(&mut *tx).await.map_err(err)?
                };
                Reply::Terminal(terminal(r))
            }

            Action::TaxManual {
                account_id,
                mut data,
                tax,
            } => {
                if data.description.trim().is_empty() {
                    return Err(invalid("cash_tax_description"));
                }
                let snapshot = tax_snapshot(
                    &mut tx,
                    c,
                    &TaxRequest {
                        amount: data.amount.clone(),
                        tax,
                    },
                )
                .await?;
                if snapshot.tax.tax_on > data.booked_on {
                    return Err(invalid("cash_tax_date"));
                }
                let value = amount(&snapshot.amounts.gross).ok_or(Error::Unavailable)?;
                data.amount = money(if snapshot.input_amount.starts_with('-') {
                    -value
                } else {
                    value
                });
                data.normalize();
                let m = insert_movement(&mut tx, c, &account_id, &data).await?;
                sqlx::query("UPDATE cash_movement SET tax=$3 WHERE company_id=$1 AND id=$2")
                    .bind(c)
                    .bind(m)
                    .bind(serde_json::to_value(snapshot).map_err(err)?)
                    .execute(&mut *tx)
                    .await
                    .map_err(err)?;
                Reply::Movement(movement(&mut tx, c, m).await?)
            }

            Action::Settings {
                account_id,
                revision,
            } => {
                let current = read_settings(&mut tx, c).await?;
                if current.revision != revision {
                    return Err(Error::Conflict);
                }
                if let Some(ref account_id) = account_id {
                    if get_account(&mut tx, c, account_id).await?.archived {
                        return Err(Error::Conflict);
                    }
                }
                sqlx::query("INSERT INTO cash_settings(company_id,account_id) VALUES($1,$2) ON CONFLICT(company_id) DO UPDATE SET account_id=$2,revision=cash_settings.revision+1").bind(c).bind(account_id.as_deref().map(id).transpose()?).execute(&mut *tx).await.map_err(err)?;
                Reply::Settings(read_settings(&mut tx, c).await?)
            }
            Action::Account {
                id: existing,
                revision,
                mut data,
                archived,
            } => {
                data.normalize();
                let e = data.validate();
                if !e.is_empty() {
                    return Err(Error::Invalid(e));
                }
                let account_id = if let Some(existing) = existing {
                    let old = get_account(&mut tx, c, &existing).await?;
                    if archived
                        && read_settings(&mut tx, c).await?.account_id.as_deref() == Some(&existing)
                    {
                        return Err(invalid("cash_default_archived"));
                    }
                    if Some(old.revision) != revision {
                        return Err(Error::Conflict);
                    }
                    let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cash_movement WHERE company_id=$1 AND account_id=$2)").bind(c).bind(id(&existing)?).fetch_one(&mut *tx).await.map_err(err)?;
                    if used
                        && (old.data.opening_on != data.opening_on
                            || amount(&old.data.opening_balance) != amount(&data.opening_balance))
                    {
                        return Err(Error::Conflict);
                    }
                    id(&existing)?
                } else {
                    if revision.is_some() {
                        return Err(invalid("revision"));
                    }
                    0
                };
                let row=if account_id==0 {
                sqlx::query("INSERT INTO cash_account(company_id,name,opening_on,opening_balance,currency,archived) VALUES($1,$2,$3,$4,$5,$6) RETURNING *")
                    .bind(c).bind(data.name).bind(data.opening_on).bind(amount(&data.opening_balance).ok_or_else(||invalid("opening_balance"))?).bind(data.currency).bind(archived).fetch_one(&mut *tx).await
            }else{
                sqlx::query("UPDATE cash_account SET name=$3,opening_on=$4,opening_balance=$5,currency=$6,archived=$7,revision=revision+1 WHERE company_id=$1 AND id=$2 RETURNING *")
                    .bind(c).bind(account_id).bind(data.name).bind(data.opening_on).bind(amount(&data.opening_balance).ok_or_else(||invalid("opening_balance"))?).bind(data.currency).bind(archived).fetch_one(&mut *tx).await
            }.map_err(|e|if e.as_database_error().is_some_and(|v|v.is_unique_violation()){Error::Conflict}else{err(e)})?;
                Reply::Account(account(row))
            }
            Action::Manual {
                account_id,
                mut data,
            } => {
                let e = data.validate();
                if !e.is_empty() {
                    return Err(Error::Invalid(e));
                }
                data.normalize();
                let m = insert_movement(&mut tx, c, &account_id, &data).await?;
                Reply::Movement(movement(&mut tx, c, m).await?)
            }
            Action::Match {
                movement_id,
                revision,
                target_kind,
                document_id,
                target_revision,
                payment_id,
            } => Reply::Movement(
                matching::match_one(
                    &mut tx,
                    actor,
                    company,
                    a,
                    c,
                    Action::Match {
                        movement_id,
                        revision,
                        target_kind,
                        document_id,
                        target_revision,
                        payment_id,
                    },
                )
                .await?,
            ),
            Action::Cancel {
                movement_id,
                revision,
                reason,
            } => {
                if reason.trim().is_empty()
                    || reason.chars().count() > 500
                    || reason.chars().any(char::is_control)
                {
                    return Err(invalid("reason"));
                }
                let m = movement(&mut tx, c, id(&movement_id)?).await?;
                if m.retail_posting_id.is_some()
                    || m.automatic
                    || m.revision != revision
                    || m.cancelled
                    || m.matches.iter().any(|v| v.detached_at.is_none())
                {
                    return Err(Error::Conflict);
                }
                sqlx::query("UPDATE cash_movement SET cancelled=true,reason=$3,revision=revision+1 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&m.id)?).bind(reason).execute(&mut *tx).await.map_err(err)?;
                Reply::Movement(movement(&mut tx, c, id(&m.id)?).await?)
            }
            Action::Unmatch {
                movement_id,
                revision,
                reason,
            } => {
                if reason.trim().is_empty()
                    || reason.chars().count() > 500
                    || reason.chars().any(char::is_control)
                {
                    return Err(invalid("reason"));
                }
                let m = movement(&mut tx, c, id(&movement_id)?).await?;
                if m.retail_posting_id.is_some() || m.automatic {
                    return Err(invalid("cash_automatic_source"));
                }
                if m.revision != revision {
                    return Err(Error::Conflict);
                }
                let link = m
                    .matches
                    .iter()
                    .find(|m| m.detached_at.is_none())
                    .ok_or(Error::Conflict)?;
                sqlx::query("UPDATE cash_match SET detached_at=now(),detached_by=$3,reason=$4 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&link.id)?).bind(a).bind(&reason).execute(&mut *tx).await.map_err(err)?;
                if link.created_payment {
                    if matches!(link.target_kind.as_str(), "advance" | "credit") {
                        crate::treasury_documents::reverse(
                            &mut tx,
                            actor,
                            company,
                            &link.target_kind,
                            &link.document_id,
                            &link.payment_id,
                            &reason,
                        )
                        .await?;
                    } else if link.target_kind == "invoice" {
                        let current =
                            crate::payment::load(&mut tx, c, id(&link.document_id)?).await?;
                        crate::payment::reverse_in_tx(
                            &mut tx,
                            actor,
                            company,
                            &link.document_id,
                            &link.payment_id,
                            payment::Reverse {
                                request_id: uuid::Uuid::new_v4().to_string(),
                                revision: current.summary.revision,
                                reason,
                                replacement: None,
                            },
                        )
                        .await?;
                    } else {
                        let current =
                            crate::received::load(&mut tx, c, id(&link.document_id)?).await?;
                        crate::received::command_in_tx(
                            &mut tx,
                            actor,
                            company,
                            &link.document_id,
                            received::Command {
                                request_id: uuid::Uuid::new_v4().to_string(),
                                revision: current.revision,
                                action: received::Action::Reverse {
                                    payment_id: link.payment_id.clone(),
                                    reason,
                                },
                            },
                        )
                        .await?;
                    }
                }
                bump(&mut tx, c, id(&m.id)?).await?;
                Reply::Movement(movement(&mut tx, c, id(&m.id)?).await?)
            }
        };
        sqlx::query("INSERT INTO cash_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(&cmd.request_id).bind(a).bind(input).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}
async fn bump(tx: &mut Transaction<'_, Postgres>, c: i64, m: i64) -> Result<()> {
    sqlx::query("UPDATE cash_movement SET revision=revision+1 WHERE company_id=$1 AND id=$2")
        .bind(c)
        .bind(m)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok(())
}

async fn read_settings(tx: &mut Transaction<'_, Postgres>, c: i64) -> Result<Settings> {
    let r = sqlx::query("SELECT account_id,revision FROM cash_settings WHERE company_id=$1")
        .bind(c)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?;
    Ok(r.map(|r| Settings {
        account_id: r.get::<Option<i64>, _>("account_id").map(|v| v.to_string()),
        revision: r.get("revision"),
    })
    .unwrap_or(Settings {
        account_id: None,
        revision: 0,
    }))
}

async fn tax_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    request: &TaxRequest,
) -> Result<TaxSnapshot> {
    let settings: serde_json::Value =
        sqlx::query_scalar("SELECT settings FROM company WHERE id=$1")
            .bind(c)
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    if settings["tax_mode"] != "vat_payer" {
        return Err(invalid("cash_tax_payer"));
    }
    let effective = settings["tax_effective_from"].as_str().unwrap_or_default();
    if !effective.is_empty()
        && effective.parse::<chrono::NaiveDate>().map_err(err)? > request.tax.tax_on
    {
        return Err(invalid("cash_tax_date"));
    }
    let valid: bool = sqlx::query_scalar("SELECT $1 <= (now() AT TIME ZONE 'Europe/Prague')::date")
        .bind(request.tax.tax_on)
        .fetch_one(&mut **tx)
        .await
        .map_err(err)?;
    if !valid {
        return Err(invalid("cash_tax_date"));
    }
    faktura_domain::cash::calculate_tax(request).map_err(Error::Invalid)
}

fn terminal(r: sqlx::postgres::PgRow) -> Terminal {
    Terminal {
        id: r.get::<i64, _>("id").to_string(),
        revision: r.get("revision"),
        code: r.get("code"),
        name: r.get("name"),
        archived: r.get("archived"),
    }
}

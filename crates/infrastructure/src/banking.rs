#[path = "bank_automation.rs"]
mod automation;
#[path = "bank_foreign.rs"]
mod foreign;
#[path = "bank_matching.rs"]
mod matching;
use async_trait::async_trait;
use chrono::Datelike;
use faktura_application::{banking::*, payment, received};
use faktura_domain::{
    payment::PaymentInput,
    received::{amount, money, Kind},
};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::HashMap;
#[derive(Clone)]
pub struct PgBank(pub PgPool);
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
            iban: r.get("iban"),
            currency: r.get("currency"),
        },
    }
}
pub(crate) async fn get_account(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    a: &str,
) -> Result<Account> {
    Ok(account(
        sqlx::query("SELECT * FROM bank_account WHERE company_id=$1 AND id=$2 FOR SHARE")
            .bind(c)
            .bind(id(a)?)
            .fetch_optional(&mut **tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?,
    ))
}
async fn movement(tx: &mut Transaction<'_, Postgres>, c: i64, m: i64) -> Result<Movement> {
    let r = sqlx::query("SELECT * FROM bank_movement WHERE company_id=$1 AND id=$2 FOR UPDATE")
        .bind(c)
        .bind(m)
        .fetch_optional(&mut **tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
    let matches: Vec<Match>=sqlx::query("SELECT b.*,coalesce(i.number,r.number,s.number,n.number) AS document_number,CASE WHEN to_jsonb(b)->>'advance_id' IS NOT NULL THEN 'advance' WHEN to_jsonb(b)->>'credit_id' IS NOT NULL THEN 'credit' ELSE coalesce(r.kind,'invoice') END AS document_kind FROM bank_match b LEFT JOIN invoice_draft i ON i.company_id=b.company_id AND i.id=b.invoice_id LEFT JOIN received_invoice r ON r.company_id=b.company_id AND r.id=b.received_id LEFT JOIN sales_document s ON s.company_id=b.company_id AND s.id=(to_jsonb(b)->>'advance_id')::bigint LEFT JOIN credit_note n ON n.company_id=b.company_id AND n.id=(to_jsonb(b)->>'credit_id')::bigint WHERE b.company_id=$1 AND b.movement_id=$2 ORDER BY b.id")
      .bind(c).bind(m).fetch_all(&mut **tx).await.map_err(err)?.into_iter().map(|r|{
        let invoice:Option<i64>=r.get("invoice_id");
        Ok(Match{foreign:r.try_get::<Option<serde_json::Value>,_>("foreign_settlement").unwrap_or(None).map(serde_json::from_value).transpose().map_err(err)?,amount:money(r.get("amount")),id:r.get::<i64,_>("id").to_string(),target_kind:if invoice.is_some(){"invoice"}else if r.try_get::<Option<i64>,_>("advance_id").unwrap_or(None).is_some(){"advance"}else if r.try_get::<Option<i64>,_>("credit_id").unwrap_or(None).is_some(){"credit"}else{"received"}.into(),
            document_id:invoice.or(r.get("received_id")).or(r.try_get("advance_id").unwrap_or(None)).or(r.try_get("credit_id").unwrap_or(None)).expect("match target constraint").to_string(),document_number:r.get("document_number"),document_kind:r.get("document_kind"),
            payment_id:r.get("payment_id"),created_payment:r.get("created_payment"),created_at:r.get::<chrono::DateTime<chrono::Utc>,_>("created_at").to_rfc3339(),
            actor_id:r.get::<i64,_>("actor_id").to_string(),detached_at:r.get::<Option<chrono::DateTime<chrono::Utc>>,_>("detached_at").map(|v|v.to_rfc3339()),
            detached_by:r.get::<Option<i64>,_>("detached_by").map(|v|v.to_string()),reason:r.get("reason")})
      }).collect::<Result<_>>()?;
    let allocated: rust_decimal::Decimal = matches
        .iter()
        .filter(|v| v.detached_at.is_none())
        .map(|v| amount(&v.amount).unwrap())
        .sum();
    Ok(Movement {
        allocated: money(allocated),
        remaining: money(r.get::<rust_decimal::Decimal, _>("amount").abs() - allocated),
        id: m.to_string(),
        account_id: r.get::<i64, _>("account_id").to_string(),
        number: r.get("number"),
        revision: r.get("revision"),
        source: r.get("source"),
        data: serde_json::from_value(r.get("data")).map_err(err)?,
        matches,
    })
}
fn parse(csv: &str) -> Result<Vec<MovementData>> {
    if csv.is_empty() || csv.len() > 1024 * 1024 {
        return Err(invalid("csv_size"));
    }
    let csv = csv.trim_start_matches('\u{feff}');
    let separator = if csv.lines().next().unwrap_or_default().contains(';') {
        b';'
    } else {
        b','
    };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(csv.as_bytes());
    let expected = [
        "external_id",
        "booked_on",
        "amount",
        "currency",
        "variable_symbol",
        "counterparty",
        "description",
    ];
    let headers = reader.headers().map_err(|_| invalid("csv_headers"))?;
    if headers.len() != expected.len()
        || !expected
            .iter()
            .all(|v| headers.iter().filter(|h| h == v).count() == 1)
    {
        return Err(invalid("csv_headers"));
    }
    let mut rows = vec![];
    for result in reader.deserialize::<MovementData>() {
        if rows.len() >= 1000 {
            return Err(invalid("csv_rows"));
        }
        let mut data = result.map_err(|_| invalid("csv_row"))?;
        // Validate lexical precision before canonicalization.
        if amount(&data.amount).is_none() {
            return Err(invalid("amount"));
        }
        data.normalize();
        rows.push(data);
    }
    if rows.is_empty() {
        return Err(invalid("csv_rows"));
    }
    Ok(rows)
}
async fn preview(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    f: FileInput,
) -> Result<ImportPreview> {
    let a = get_account(tx, c, &f.account_id).await?;
    if a.archived {
        return Err(Error::Conflict);
    }
    let data = parse(&f.csv)?;
    preview_rows(tx, c, a, data, "csv", 2).await
}
pub(crate) async fn preview_rows(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    a: Account,
    data: Vec<MovementData>,
    source: &str,
    first_row: usize,
) -> Result<ImportPreview> {
    if a.archived {
        return Err(Error::Conflict);
    }
    let mixed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bank_movement WHERE company_id=$1 AND account_id=$2 AND source NOT IN ('manual',$3))")
        .bind(c).bind(id(&a.id)?).bind(source).fetch_one(&mut **tx).await.map_err(err)?;
    if mixed {
        return Err(invalid("statement_mixed_format"));
    }
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    let mut seen = HashMap::new();
    let mut rows = vec![];
    let mut new_count = 0;
    let mut duplicate_count = 0;
    for (n, d) in data.into_iter().enumerate() {
        let mut fields = d.validate();
        if d.currency != a.data.currency {
            fields.push("currency".into());
        }
        if d.booked_on > today {
            fields.push("booked_on".into());
        }
        let stored=sqlx::query_scalar::<_,serde_json::Value>("SELECT data FROM bank_movement WHERE company_id=$1 AND account_id=$2 AND external_id=$3")
            .bind(c).bind(id(&a.id)?).bind(&d.external_id).fetch_optional(&mut **tx).await.map_err(err)?;
        let status = if !fields.is_empty() {
            "invalid"
        } else if let Some(previous) = seen.get(&d.external_id).cloned().or(stored) {
            if previous == serde_json::to_value(&d).map_err(err)? {
                duplicate_count += 1;
                "duplicate"
            } else {
                "conflict"
            }
        } else {
            new_count += 1;
            "new"
        };
        seen.insert(
            d.external_id.clone(),
            serde_json::to_value(&d).map_err(err)?,
        );
        rows.push(PreviewRow {
            row: n + first_row,
            data: d,
            status: status.into(),
            fields,
        });
    }
    Ok(ImportPreview {
        valid: rows
            .iter()
            .all(|r| ["new", "duplicate"].contains(&r.status.as_str())),
        rows,
        new_count,
        duplicate_count,
    })
}
async fn statement_preview(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    f: StatementInput,
) -> Result<ImportPreview> {
    let a = get_account(tx, c, &f.account_id).await?;
    let data = crate::bank_statement::parse(&f, &a.data.iban, &a.data.currency)?;
    preview_rows(tx, c, a, data, &f.format, 1).await
}
pub(crate) async fn insert_movement(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    account: &str,
    data: &MovementData,
    source: &str,
) -> Result<i64> {
    let a = get_account(tx, c, account).await?;
    if a.archived {
        return Err(Error::Conflict);
    }
    let mut e = data.validate();
    if data.currency != a.data.currency {
        e.push("currency".into());
    }
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
    let year = data.booked_on.year();
    let seq:i64=sqlx::query_scalar("INSERT INTO bank_series(company_id,year,next_value) VALUES($1,$2,2) ON CONFLICT(company_id,year) DO UPDATE SET next_value=bank_series.next_value+1 RETURNING next_value-1").bind(c).bind(year).fetch_one(&mut **tx).await.map_err(err)?;
    sqlx::query_scalar("INSERT INTO bank_movement(company_id,account_id,number,source,external_id,data,amount,booked_on) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id")
      .bind(c).bind(id(account)?).bind(format!("B{year}{seq:06}")).bind(source).bind(&data.external_id).bind(serde_json::to_value(data).map_err(err)?).bind(amount(&data.amount).ok_or_else(||invalid("amount"))?).bind(data.booked_on).fetch_one(&mut **tx).await.map_err(err)
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
    sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM bank_match WHERE company_id=$1 AND {column}=$2 AND payment_id=$3 AND detached_at IS NULL)")).bind(c).bind(doc).bind(payment).fetch_one(&mut **tx).await.map_err(err)
}
async fn candidates(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    m: &Movement,
    q: &str,
    only: Option<i64>,
    partial: bool,
) -> Result<Vec<Candidate>> {
    if q.len() > 200 {
        return Err(invalid("query"));
    }
    if !partial && m.matches.iter().any(|v| v.detached_at.is_none()) {
        return Ok(vec![]);
    }
    let a = get_account(tx, c, &m.account_id).await?;
    if a.archived {
        return Ok(vec![]);
    }
    let value = amount(&m.data.amount).ok_or(Error::Unavailable)?;
    let incoming = value.is_sign_positive();
    let value = if partial {
        amount(&m.remaining).ok_or(Error::Unavailable)?
    } else {
        value.abs()
    };
    if value <= rust_decimal::Decimal::ZERO {
        return Ok(vec![]);
    }
    let mut result = vec![];
    let table = if incoming {
        "invoice_draft"
    } else {
        "received_invoice"
    };
    let filter = if incoming {
        "number IS NOT NULL"
    } else {
        "NOT cancelled AND (data->>'currency'='CZK' OR kind='invoice')"
    };
    let partner = if incoming { "customer" } else { "supplier" };
    let currency_path = if incoming {
        "data->'calculation'->>'currency'"
    } else {
        "data->>'currency'"
    };
    let rows=sqlx::query(&format!("SELECT id,number,data FROM {table} WHERE company_id=$1 AND ($3::bigint IS NULL OR id=$3) AND {currency_path}=$4 AND {filter} AND ($2='' OR position(lower($2) in lower(number||' '||coalesce(data->'{partner}'->>'name','')||' '||coalesce(data->>'external_number','')||' '||coalesce(data->'payment'->>'variable_symbol',data->>'variable_symbol','')))>0) ORDER BY id DESC LIMIT 50"))
        .bind(c).bind(q).bind(only).bind(&m.data.currency).fetch_all(&mut **tx).await.map_err(err)?;
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
        if currency != Some(m.data.currency.as_str()) {
            continue;
        }
        if incoming {
            let payments = crate::payment::load(tx, c, doc).await?;
            let eligible_date = json["issued_on"]
                .as_str()
                .is_some_and(|d| d <= m.data.booked_on.to_string().as_str());
            if eligible_date
                && amount(&payments.summary.outstanding)
                    .is_some_and(|v| v > rust_decimal::Decimal::ZERO && (partial || v >= value))
            {
                result.push(Candidate {
                    target_kind: "invoice".into(),
                    document_kind: "invoice".into(),
                    document_id: doc.to_string(),
                    number: number.clone(),
                    partner: name.clone(),
                    revision: payments.summary.revision,
                    payment_id: None,
                    available: money(amount(&payments.summary.outstanding).unwrap().min(value)),
                    outstanding: payments.summary.outstanding.clone(),
                });
            }
            for p in payments.items.iter().filter(|p| {
                p.reversed_at.is_none()
                    && p.method == "bank_transfer"
                    && amount(&p.amount)
                        .is_some_and(|v| if partial { v <= value } else { v == value })
            }) {
                if !linked(tx, c, "invoice", doc, &p.id).await? {
                    result.push(Candidate {
                        target_kind: "invoice".into(),
                        document_kind: "invoice".into(),
                        document_id: doc.to_string(),
                        number: number.clone(),
                        partner: name.clone(),
                        revision: payments.summary.revision,
                        payment_id: Some(p.id.clone()),
                        available: p.amount.clone(),
                        outstanding: payments.summary.outstanding.clone(),
                    });
                }
            }
        } else {
            let d = crate::received::load(tx, c, doc).await?;
            if d.data.kind == Kind::Invoice
                && d.data.issued_on <= m.data.booked_on
                && amount(&d.remaining)
                    .is_some_and(|v| v > rust_decimal::Decimal::ZERO && (partial || v >= value))
            {
                result.push(Candidate {
                    target_kind: "received".into(),
                    document_kind: "invoice".into(),
                    document_id: doc.to_string(),
                    number: number.clone(),
                    partner: name.clone(),
                    revision: d.revision,
                    payment_id: None,
                    available: money(amount(&d.remaining).unwrap().min(value)),
                    outstanding: d.remaining.clone(),
                });
            }
            for p in d.payments.iter().filter(|p| {
                p.reversed_at.is_none()
                    && p.input.method
                        == if d.data.kind == Kind::Receipt {
                            "card"
                        } else {
                            "bank_transfer"
                        }
                    && amount(&p.input.amount).is_some_and(|v| {
                        if partial {
                            v <= value
                        } else {
                            v == value
                        }
                    })
            }) {
                if !linked(tx, c, "received", doc, &p.id).await? {
                    result.push(Candidate {
                        target_kind: "received".into(),
                        document_kind: d.data.kind.as_str().into(),
                        document_id: doc.to_string(),
                        number: number.clone(),
                        partner: name.clone(),
                        revision: d.revision,
                        payment_id: Some(p.id.clone()),
                        available: p.input.amount.clone(),
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
                "bank_transfer",
                &m.data.currency,
                q,
                None,
                partial,
            )
            .await?,
        );
    }
    Ok(result)
}
#[async_trait]
impl BankStore for PgBank {
    async fn foreign_report(
        &self,
        a: &str,
        c: &str,
        q: ForeignReportQuery,
    ) -> Result<ForeignReport> {
        faktura_application::purchase_report::Period {
            from: q.from,
            to: q.to,
        }
        .validate()?;
        if (q.to - q.from).num_days() > 3653 || !(1..=10000).contains(&q.page.unwrap_or(1)) {
            return Err(invalid("period"));
        }
        let mut tx = self
            .0
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ")
            .await
            .map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        foreign::report(&mut tx, c, q).await
    }
    async fn foreign_candidates(
        &self,
        a: &str,
        c: &str,
        m: &str,
        q: &str,
    ) -> Result<Vec<ForeignCandidate>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let m = movement(&mut tx, c, id(m)?).await?;
        foreign::candidates(&mut tx, c, &m, q, None).await
    }
    async fn balance(&self, a: &str, c: &str, q: BalanceQuery) -> Result<Balance> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let account = get_account(&mut tx, c, &q.account_id).await?;
        let row = sqlx::query(
            "SELECT opening_on,opening_balance FROM bank_account WHERE company_id=$1 AND id=$2",
        )
        .bind(c)
        .bind(id(&q.account_id)?)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let opening_on: Option<chrono::NaiveDate> = row.get("opening_on");
        let configured: Option<rust_decimal::Decimal> = row.get("opening_balance");
        if matches!((q.from,q.to),(Some(f),Some(t)) if f>t)
            || matches!((opening_on,q.from),(Some(o),Some(f)) if f<o)
            || matches!((opening_on,q.to),(Some(o),Some(t)) if t<o)
        {
            return Err(invalid("balance_date"));
        }
        let from = q.from.or(opening_on);
        let row=sqlx::query("SELECT coalesce(sum(amount) FILTER(WHERE $3::date IS NOT NULL AND booked_on<$3),0) AS before,coalesce(sum(amount) FILTER(WHERE ($3::date IS NULL OR booked_on >= $3) AND amount>0),0) AS income,coalesce(-sum(amount) FILTER(WHERE ($3::date IS NULL OR booked_on >= $3) AND amount<0),0) AS expense,count(*) FILTER(WHERE $3::date IS NULL OR booked_on >= $3) AS count FROM bank_movement WHERE company_id=$1 AND account_id=$2 AND ($4::date IS NULL OR booked_on <= $4) AND ($5::date IS NULL OR booked_on >= $5)").bind(c).bind(id(&q.account_id)?).bind(from).bind(q.to).bind(opening_on).fetch_one(&mut *tx).await.map_err(err)?;
        let income: rust_decimal::Decimal = row.get("income");
        let expense: rust_decimal::Decimal = row.get("expense");
        let opening = configured.map(|v| v + row.get::<rust_decimal::Decimal, _>("before"));
        Ok(Balance {
            currency: account.data.currency,
            opening_on,
            opening_balance: configured.map(money),
            opening: opening.map(money),
            income: money(income),
            expense: money(expense),
            closing: opening.map(|v| money(v + income - expense)),
            count: row.get("count"),
        })
    }

    async fn accounts(&self, a: &str, c: &str) -> Result<Vec<Account>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        Ok(
            sqlx::query("SELECT * FROM bank_account WHERE company_id=$1 ORDER BY archived,name,id")
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
    async fn preview(&self, a: &str, c: &str, f: FileInput) -> Result<ImportPreview> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        preview(&mut tx, c, f).await
    }
    async fn candidates(&self, a: &str, c: &str, m: &str, q: &str) -> Result<Vec<Candidate>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let m = movement(&mut tx, c, id(m)?).await?;
        candidates(&mut tx, c, &m, q, None, false).await
    }
    async fn movements(&self, a: &str, c: &str, q: Search) -> Result<Page> {
        if q.query.len() > 200
            || !(1..=10000).contains(&q.page.unwrap_or(1))
            || !["", "all", "income", "expense"].contains(&q.direction.as_str())
            || !["", "all", "matched", "partial", "unmatched"].contains(&q.status.as_str())
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
        let predicate="company_id=$1 AND ($2::bigint IS NULL OR account_id=$2) AND CASE $3 WHEN 'income' THEN amount>0 WHEN 'expense' THEN amount<0 ELSE true END AND ($4='' OR position(lower($4) in lower(number||' '||(data->>'variable_symbol')||' '||(data->>'counterparty')||' '||(data->>'description')))>0) AND CASE $5 WHEN 'matched' THEN abs(m.amount)=(SELECT coalesce(sum(b.amount),0) FROM bank_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL) WHEN 'partial' THEN (SELECT coalesce(sum(b.amount),0) FROM bank_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL)>0 AND abs(m.amount)>(SELECT coalesce(sum(b.amount),0) FROM bank_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL) WHEN 'unmatched' THEN NOT EXISTS(SELECT 1 FROM bank_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL) ELSE true END";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM bank_movement m WHERE {predicate}"
        ))
        .bind(c)
        .bind(account_id)
        .bind(&q.direction)
        .bind(&q.query)
        .bind(&q.status)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let ids:Vec<i64>=sqlx::query_scalar(&format!("SELECT id FROM bank_movement m WHERE {predicate} ORDER BY booked_on DESC,id DESC LIMIT 25 OFFSET $6")).bind(c).bind(account_id).bind(q.direction).bind(q.query).bind(q.status).bind((q.page.unwrap_or(1)-1)*25).fetch_all(&mut *tx).await.map_err(err)?;
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
    async fn statement_preview(
        &self,
        a: &str,
        c: &str,
        f: StatementInput,
    ) -> Result<ImportPreview> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        statement_preview(&mut tx, c, f).await
    }
    async fn split_candidates(&self, a: &str, c: &str, m: &str, q: &str) -> Result<Vec<Candidate>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let m = movement(&mut tx, c, id(m)?).await?;
        candidates(&mut tx, c, &m, q, None, true).await
    }
    async fn suggestions(
        &self,
        a: &str,
        company: &str,
        movement_id: &str,
    ) -> Result<Vec<Candidate>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, company, false).await?;
        let m = movement(&mut tx, c, id(movement_id)?).await?;
        matching::suggestions(&mut tx, c, &m).await
    }
    async fn batch_preview(&self, a: &str, c: &str, input: BatchInput) -> Result<BatchPreview> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        matching::batch_preview(&mut tx, c, input).await
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
            "SELECT * FROM bank_command WHERE company_id=$1 AND request_id=$2::text::uuid",
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
            Action::Opening {
                account_id,
                revision,
                opening_on,
                opening_balance,
                reason,
            } => {
                let previous = get_account(&mut tx, c, &account_id).await?;
                if previous.revision != revision || previous.archived {
                    return Err(Error::Conflict);
                }
                let value = amount(&opening_balance).ok_or_else(|| invalid("opening_balance"))?;
                if reason.trim().is_empty()
                    || reason.chars().count() > 500
                    || reason.chars().any(char::is_control)
                {
                    return Err(invalid("reason"));
                }
                let valid:bool=sqlx::query_scalar("SELECT $1 >= DATE '1900-01-01' AND $1 <= (now() AT TIME ZONE 'Europe/Prague')::date").bind(opening_on).fetch_one(&mut *tx).await.map_err(err)?;
                if !valid {
                    return Err(invalid("balance_date"));
                }
                let row=sqlx::query("UPDATE bank_account SET opening_on=$3,opening_balance=$4,revision=revision+1 WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(&account_id)?).bind(opening_on).bind(value).fetch_one(&mut *tx).await.map_err(err)?;
                Reply::Account(account(row))
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
                    if Some(old.revision) != revision {
                        return Err(Error::Conflict);
                    }
                    let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bank_movement WHERE company_id=$1 AND account_id=$2)").bind(c).bind(id(&existing)?).fetch_one(&mut *tx).await.map_err(err)?;
                    if old.data.currency != data.currency || (used && old.data.iban != data.iban) {
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
                sqlx::query("INSERT INTO bank_account(company_id,name,iban,currency,archived) VALUES($1,$2,$3,$4,$5) RETURNING *")
                    .bind(c).bind(data.name).bind(data.iban).bind(data.currency).bind(archived).fetch_one(&mut *tx).await
            }else{
                sqlx::query("UPDATE bank_account SET name=$3,iban=$4,currency=$5,archived=$6,revision=revision+1 WHERE company_id=$1 AND id=$2 RETURNING *")
                    .bind(c).bind(account_id).bind(data.name).bind(data.iban).bind(data.currency).bind(archived).fetch_one(&mut *tx).await
            }.map_err(|e|if e.as_database_error().is_some_and(|v|v.is_unique_violation()){Error::Conflict}else{err(e)})?;
                Reply::Account(account(row))
            }
            Action::Import { account_id, csv } => {
                let report = preview(
                    &mut tx,
                    c,
                    FileInput {
                        account_id: account_id.clone(),
                        csv,
                    },
                )
                .await?;
                if !report.valid {
                    return Err(invalid("import_conflict"));
                }
                for r in report.rows.iter().filter(|r| r.status == "new") {
                    insert_movement(&mut tx, c, &account_id, &r.data, "csv").await?;
                }
                Reply::Import(report)
            }
            Action::Statement { file } => {
                let account_id = file.account_id.clone();
                let source = file.format.clone();
                let report = statement_preview(&mut tx, c, file).await?;
                if !report.valid {
                    return Err(invalid("import_conflict"));
                }
                for r in report.rows.iter().filter(|r| r.status == "new") {
                    insert_movement(&mut tx, c, &account_id, &r.data, &source).await?;
                }
                Reply::Import(report)
            }
            Action::Manual {
                account_id,
                mut data,
            } => {
                data.external_id = format!("manual:{}", cmd.request_id);
                let e = data.validate();
                if !e.is_empty() {
                    return Err(Error::Invalid(e));
                }
                data.normalize();
                let m = insert_movement(&mut tx, c, &account_id, &data, "manual").await?;
                Reply::Movement(movement(&mut tx, c, m).await?)
            }
            action @ Action::ForeignMatch { .. } => {
                Reply::Movement(foreign::execute(&mut tx, actor, company, a, c, action).await?)
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
            Action::Split {
                movement_id,
                revision,
                allocations,
            } => Reply::Movement(
                matching::split(
                    &mut tx,
                    actor,
                    company,
                    a,
                    c,
                    &movement_id,
                    revision,
                    allocations,
                )
                .await?,
            ),
            Action::BatchMatch {
                movement_ids,
                fingerprint,
            } => {
                if fingerprint.len() != 64 || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(invalid("fingerprint"));
                }
                let report =
                    matching::batch_preview(&mut tx, c, BatchInput { movement_ids }).await?;
                if report.fingerprint != fingerprint {
                    return Err(Error::Conflict);
                }
                if report.ready_count == 0 {
                    return Err(invalid("batch_empty"));
                }
                Reply::Batch(matching::apply_batch(&mut tx, actor, company, a, c, report).await?)
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
                if m.revision != revision {
                    return Err(Error::Conflict);
                }
                let links: Vec<_> = m
                    .matches
                    .iter()
                    .filter(|v| v.detached_at.is_none())
                    .collect();
                if links.is_empty() {
                    return Err(Error::Conflict);
                }
                for link in links {
                    sqlx::query("UPDATE bank_match SET detached_at=now(),detached_by=$3,reason=$4 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&link.id)?).bind(a).bind(&reason).execute(&mut *tx).await.map_err(err)?;
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
                                    reason: reason.clone(),
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
                                        reason: reason.clone(),
                                    },
                                },
                            )
                            .await?;
                        }
                    }
                }
                bump(&mut tx, c, id(&m.id)?).await?;
                Reply::Movement(movement(&mut tx, c, id(&m.id)?).await?)
            }
        };
        sqlx::query("INSERT INTO bank_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(&cmd.request_id).bind(a).bind(input).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}
async fn bump(tx: &mut Transaction<'_, Postgres>, c: i64, m: i64) -> Result<()> {
    sqlx::query("UPDATE bank_movement SET revision=revision+1 WHERE company_id=$1 AND id=$2")
        .bind(c)
        .bind(m)
        .execute(&mut **tx)
        .await
        .map_err(err)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_parser() {
        let v=parse("\u{feff}external_id;booked_on;amount;currency;variable_symbol;counterparty;description\nx;2026-10-01;-12.30;CZK;001;Vendor;\"Quoted; text\"").unwrap();
        assert_eq!(v[0].amount, "-12.30");
        assert_eq!(v[0].variable_symbol, "001");
        assert!(parse("external_id,external_id\nx,x").is_err());
        assert!(parse("external_id,booked_on,amount,currency,variable_symbol,counterparty,description\nx,2026-10-01,0.001,CZK,,,").is_err());
    }
}

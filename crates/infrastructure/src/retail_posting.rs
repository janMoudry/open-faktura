use super::*;
use faktura_domain::received::{amount, money};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
async fn preview(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    input: &PostingInput,
) -> Result<PostingPreview> {
    if input.document_ids.is_empty() || input.document_ids.len() > 100 {
        return Err(invalid("documents"));
    }
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(err)?;
    if input.booked_on > today {
        return Err(invalid("booked_on"));
    }
    let mut seen = HashSet::new();
    let mut groups: BTreeMap<String, PostingGroup> = BTreeMap::new();
    let mut revisions = vec![];
    let mut ids = input.document_ids.clone();
    ids.sort();
    for i in ids {
        if !seen.insert(id(&i)?) {
            return Err(invalid("documents"));
        }
        let d = load(tx, c, &i).await?;
        if d.cancelled
            || d.posting_id.is_some()
            || d.data.invoice.issued_on.is_none_or(|v| v > input.booked_on)
        {
            return Err(Error::Conflict);
        }
        revisions.push((d.id.clone(), d.revision));
        for part in d.allocations {
            let key = serde_json::to_string(&(
                d.terminal.as_ref().map(|v| &v.id),
                &d.data.invoice.customer,
                &part.method,
            ))
            .map_err(err)?;
            let group = groups.entry(key).or_insert_with(|| PostingGroup {
                method: part.method.clone(),
                terminal: d.terminal.clone(),
                customer: d.data.invoice.customer.clone(),
                document_ids: vec![],
                paid: "0.00".into(),
                rounding: "0.00".into(),
                tax_summary: vec![],
                cash_movement_id: None,
                internal_number: None,
            });
            group.document_ids.push(d.id.clone());
            group.paid = money(
                amount(&group.paid).ok_or(Error::Unavailable)?
                    + amount(&part.paid).ok_or(Error::Unavailable)?,
            );
            group.rounding = money(
                amount(&group.rounding).ok_or(Error::Unavailable)?
                    + amount(&part.rounding).ok_or(Error::Unavailable)?,
            );
            for tax in part.tax_summary {
                if let Some(old) = group
                    .tax_summary
                    .iter_mut()
                    .find(|v| v.vat_percent == tax.vat_percent && v.tax_mode == tax.tax_mode)
                {
                    old.amounts.net = money(
                        amount(&old.amounts.net).ok_or_else(|| invalid("posting_amount"))?
                            + amount(&tax.amounts.net).ok_or_else(|| invalid("posting_amount"))?,
                    );
                    old.amounts.vat = money(
                        amount(&old.amounts.vat).ok_or_else(|| invalid("posting_amount"))?
                            + amount(&tax.amounts.vat).ok_or_else(|| invalid("posting_amount"))?,
                    );
                    old.amounts.gross = money(
                        amount(&old.amounts.gross).ok_or_else(|| invalid("posting_amount"))?
                            + amount(&tax.amounts.gross)
                                .ok_or_else(|| invalid("posting_amount"))?,
                    );
                } else {
                    group.tax_summary.push(tax);
                }
            }
        }
    }
    let groups: Vec<_> = groups.into_values().collect();
    if groups.iter().any(|v| v.method == "cash") {
        let account = input
            .cash_account_id
            .as_ref()
            .ok_or_else(|| invalid("cash_account"))?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cash_account WHERE company_id=$1 AND id=$2 AND NOT archived AND opening_on<=$3)").bind(c).bind(id(account)?).bind(input.booked_on).fetch_one(&mut **tx).await.map_err(err)?;
        if !valid {
            return Err(Error::Conflict);
        }
    }
    let bytes = serde_json::to_vec(&(input, &revisions, &groups)).map_err(err)?;
    let fingerprint = format!("{:x}", Sha256::digest(bytes));
    Ok(PostingPreview {
        groups,
        fingerprint,
    })
}
fn posting(r: sqlx::postgres::PgRow) -> Result<Posting> {
    Ok(Posting {
        id: r.get::<i64, _>("id").to_string(),
        revision: r.get("revision"),
        booked_on: r.get("booked_on"),
        groups: serde_json::from_value(r.get("groups")).map_err(err)?,
        cancelled: r.get("cancelled"),
        reason: r.get("reason"),
    })
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: &serde_json::Value,
) -> Result<Option<Posting>> {
    if uuid::Uuid::parse_str(key).is_err() {
        return Err(invalid("request_id"));
    }
    if let Some(r) = sqlx::query(
        "SELECT * FROM retail_post_command WHERE company_id=$1 AND request_id=$2::text::uuid",
    )
    .bind(c)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    {
        if r.get::<i64, _>("actor_id") != a || r.get::<serde_json::Value, _>("input") != *input {
            return Err(Error::Conflict);
        }
        return Ok(Some(serde_json::from_value(r.get("result")).map_err(err)?));
    }
    Ok(None)
}
async fn save_reply(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    key: &str,
    input: &serde_json::Value,
    value: &Posting,
) -> Result<()> {
    sqlx::query("INSERT INTO retail_post_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(key).bind(a).bind(input).bind(serde_json::to_value(value).map_err(err)?).execute(&mut **tx).await.map_err(err)?;
    Ok(())
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    a: i64,
    c: i64,
    ids: Vec<String>,
    reason: &str,
) -> Result<()> {
    for i in ids {
        let d = load(tx, c, &i).await?;
        sqlx::query("INSERT INTO retail_audit(company_id,document_id,actor_id,revision,reason,snapshot) VALUES($1,$2,$3,$4,$5,$6)").bind(c).bind(id(&i)?).bind(a).bind(d.revision).bind(reason).bind(serde_json::to_value(d).map_err(err)?).execute(&mut **tx).await.map_err(err)?;
    }
    Ok(())
}
#[async_trait]
impl PostingStore for PgRetail {
    async fn posting_pdf(&self, a: &str, c: &str, i: &str, language: &str) -> Result<Vec<u8>> {
        if !["cs", "en"].contains(&language) {
            return Err(invalid("language"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, company) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let row = sqlx::query("SELECT * FROM retail_posting WHERE company_id=$1 AND id=$2")
            .bind(company)
            .bind(id(i)?)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
        let issuer: Option<serde_json::Value> = row.get("issuer");
        let document = posting(row)?;
        let revision = document.revision;
        let cached: Option<Vec<u8>> = sqlx::query_scalar("SELECT content FROM retail_posting_pdf WHERE company_id=$1 AND posting_id=$2 AND revision=$3 AND language=$4")
            .bind(company).bind(id(i)?).bind(revision).bind(language).fetch_optional(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        if let Some(bytes) = cached {
            return Ok(bytes);
        }
        let lang = language.to_owned();
        let bytes = tokio::task::spawn_blocking(move || {
            crate::retail_posting_pdf::render(&document, issuer.as_ref(), &lang)
        })
        .await
        .map_err(err)?
        .map_err(err)?;
        sqlx::query("INSERT INTO retail_posting_pdf(company_id,posting_id,revision,language,content) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
            .bind(company).bind(id(i)?).bind(revision).bind(language).bind(&bytes).execute(&self.0).await.map_err(err)?;
        sqlx::query_scalar("SELECT content FROM retail_posting_pdf WHERE company_id=$1 AND posting_id=$2 AND revision=$3 AND language=$4")
            .bind(company).bind(id(i)?).bind(revision).bind(language).fetch_one(&self.0).await.map_err(err)
    }

    async fn posting_preview(
        &self,
        a: &str,
        c: &str,
        input: PostingInput,
    ) -> Result<PostingPreview> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        preview(&mut tx, c, &input).await
    }
    async fn postings(&self, a: &str, c: &str) -> Result<Vec<Posting>> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        sqlx::query("SELECT * FROM retail_posting WHERE company_id=$1 ORDER BY id DESC LIMIT 100")
            .bind(c)
            .fetch_all(&mut *tx)
            .await
            .map_err(err)?
            .into_iter()
            .map(posting)
            .collect()
    }
    async fn post(&self, actor: &str, company: &str, cmd: PostCommand) -> Result<Posting> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, actor, company, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(v) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        let mut preview = preview(&mut tx, c, &cmd.input).await?;
        if preview.fingerprint != cmd.fingerprint {
            return Err(Error::Conflict);
        }
        let posting_id:i64=sqlx::query_scalar("INSERT INTO retail_posting(company_id,booked_on,groups,reason,actor_id,issuer) VALUES($1,$2,'[]','post',$3,(SELECT settings FROM company WHERE id=$1)) RETURNING id").bind(c).bind(cmd.input.booked_on).bind(a).fetch_one(&mut *tx).await.map_err(err)?;
        for (index, group) in preview.groups.iter_mut().enumerate() {
            if group.method == "cash" {
                let data = faktura_domain::cash::MovementData {
                    booked_on: cmd.input.booked_on,
                    amount: group.paid.clone(),
                    currency: "CZK".into(),
                    variable_symbol: String::new(),
                    counterparty: group
                        .customer
                        .as_ref()
                        .map(|v| v.name.clone())
                        .unwrap_or_default(),
                    description: format!(
                        "PR / {} / {}",
                        posting_id,
                        group
                            .terminal
                            .as_ref()
                            .map(|t| t.code.as_str())
                            .unwrap_or("-")
                    ),
                    person: String::new(),
                };
                let m = crate::cash::insert_movement(
                    &mut tx,
                    c,
                    cmd.input
                        .cash_account_id
                        .as_deref()
                        .ok_or_else(|| invalid("cash_account"))?,
                    &data,
                )
                .await?;
                sqlx::query("UPDATE cash_movement SET retail_posting_id=$3,retail_tax=$4 WHERE company_id=$1 AND id=$2").bind(c).bind(m).bind(posting_id).bind(serde_json::to_value(&group.tax_summary).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
                group.cash_movement_id = Some(m.to_string());
            } else {
                group.internal_number = Some(format!(
                    "IN{}{:06}-{}",
                    cmd.input.booked_on.year(),
                    posting_id,
                    index + 1
                ));
            }
        }
        sqlx::query("UPDATE retail_posting SET groups=$3 WHERE company_id=$1 AND id=$2")
            .bind(c)
            .bind(posting_id)
            .bind(serde_json::to_value(&preview.groups).map_err(err)?)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        for i in &cmd.input.document_ids {
            sqlx::query("UPDATE retail_document SET posting_id=$3,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(c).bind(id(i)?).bind(posting_id).execute(&mut *tx).await.map_err(err)?;
        }
        audit(&mut tx, a, c, cmd.input.document_ids, "post").await?;
        let result = Posting {
            id: posting_id.to_string(),
            revision: 1,
            booked_on: cmd.input.booked_on,
            groups: preview.groups,
            cancelled: false,
            reason: "post".into(),
        };
        save_reply(&mut tx, a, c, &cmd.request_id, &input, &result).await?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
    async fn unpost(&self, actor: &str, company: &str, cmd: UnpostCommand) -> Result<Posting> {
        if cmd.reason.trim().is_empty()
            || cmd.reason.chars().count() > 500
            || cmd.reason.chars().any(char::is_control)
        {
            return Err(invalid("reason"));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = crate::draft::authorize(&mut tx, actor, company, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(v) = replay(&mut tx, a, c, &cmd.request_id, &input).await? {
            return Ok(v);
        }
        let mut value = posting(
            sqlx::query("SELECT * FROM retail_posting WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(c)
                .bind(id(&cmd.id)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
        )?;
        if value.cancelled || value.revision != cmd.revision {
            return Err(Error::Conflict);
        }
        sqlx::query("UPDATE cash_movement SET cancelled=true,reason=$3,revision=revision+1 WHERE company_id=$1 AND retail_posting_id=$2").bind(c).bind(id(&cmd.id)?).bind(&cmd.reason).execute(&mut *tx).await.map_err(err)?;
        let ids:Vec<i64>=sqlx::query_scalar("UPDATE retail_document SET posting_id=NULL,revision=revision+1,updated_at=now() WHERE company_id=$1 AND posting_id=$2 RETURNING id").bind(c).bind(id(&cmd.id)?).fetch_all(&mut *tx).await.map_err(err)?;
        audit(
            &mut tx,
            a,
            c,
            ids.into_iter().map(|v| v.to_string()).collect(),
            &cmd.reason,
        )
        .await?;
        sqlx::query("UPDATE retail_posting SET cancelled=true,reason=$3,revision=revision+1 WHERE company_id=$1 AND id=$2").bind(c).bind(id(&cmd.id)?).bind(&cmd.reason).execute(&mut *tx).await.map_err(err)?;
        value.cancelled = true;
        value.revision += 1;
        value.reason = cmd.reason;
        save_reply(&mut tx, a, c, &cmd.request_id, &input, &value).await?;
        tx.commit().await.map_err(err)?;
        Ok(value)
    }
}

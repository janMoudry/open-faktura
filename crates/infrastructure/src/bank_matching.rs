//! Shared matching logic for detail suggestions and atomic batch confirmation.
use super::*;
use sha2::{Digest, Sha256};
pub(super) async fn suggestions(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    m: &Movement,
) -> Result<Vec<Candidate>> {
    if m.data.variable_symbol.is_empty() || m.matches.iter().any(|v| v.detached_at.is_none()) {
        return Ok(vec![]);
    }
    let value = amount(&m.data.amount).ok_or(Error::Unavailable)?;
    let incoming = value.is_sign_positive();
    let (table, filter, symbol) = if incoming {
        (
            "invoice_draft",
            "number IS NOT NULL",
            "data->'payment'->>'variable_symbol'",
        )
    } else {
        (
            "received_invoice",
            "NOT cancelled AND kind='invoice'",
            "data->>'variable_symbol'",
        )
    };
    // Filter before limiting: unrelated recent documents cannot hide an older match.
    let ids: Vec<i64> = sqlx::query_scalar(&format!("SELECT id FROM {table} WHERE company_id=$1 AND {filter} AND coalesce({symbol},'')<>'' AND ltrim({symbol},'0')=$2 ORDER BY id LIMIT 101"))
            .bind(c).bind(m.data.variable_symbol.trim_start_matches('0')).fetch_all(&mut **tx).await.map_err(err)?;
    if ids.len() > 100 {
        return Err(invalid("suggestions_limit"));
    }
    let mut result = vec![];
    for doc in ids {
        let mut choices = candidates(tx, c, m, "", Some(doc), false).await?;
        // Linking a recorded payment takes precedence over creating a second payment.
        if choices.iter().any(|v| v.payment_id.is_some()) {
            choices.retain(|v| v.payment_id.is_some());
        }
        result.extend(choices);
    }
    if result
        .iter()
        .any(|v| v.payment_id.is_some() || amount(&v.outstanding) == Some(value.abs()))
    {
        result.retain(|v| v.payment_id.is_some() || amount(&v.outstanding) == Some(value.abs()));
    }
    Ok(result)
}
pub(super) async fn match_one(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    a: i64,
    c: i64,
    action: Action,
) -> Result<Movement> {
    match_part(tx, actor, company, a, c, action, None).await
}
#[allow(clippy::too_many_arguments)]
async fn match_part(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    a: i64,
    c: i64,
    action: Action,
    part: Option<rust_decimal::Decimal>,
) -> Result<Movement> {
    let Action::Match {
        movement_id,
        revision,
        target_kind,
        document_id,
        target_revision,
        payment_id,
    } = action
    else {
        return Err(invalid("action"));
    };
    let mut m = movement(tx, c, id(&movement_id)?).await?;
    if m.revision != revision
        || (part.is_none() && m.matches.iter().any(|m| m.detached_at.is_none()))
    {
        return Err(Error::Conflict);
    }
    let allocated = part.unwrap_or(amount(&m.data.amount).ok_or(Error::Unavailable)?.abs());
    if allocated <= rust_decimal::Decimal::ZERO
        || allocated > amount(&m.remaining).ok_or(Error::Unavailable)?
    {
        return Err(invalid("split_amount"));
    }
    if part.is_some() {
        m.data.amount = money(if m.data.amount.starts_with('-') {
            -allocated
        } else {
            allocated
        });
        m.matches.clear();
    }
    if get_account(tx, c, &m.account_id).await?.archived {
        return Err(Error::Conflict);
    }
    if matches!(target_kind.as_str(), "advance" | "credit") {
        let incoming = !m.data.amount.starts_with('-');
        let valid = crate::treasury_documents::candidates(
            tx,
            c,
            incoming,
            allocated,
            m.data.booked_on,
            "bank_transfer",
            "",
            Some(id(&document_id)?),
            false,
        )
        .await?
        .into_iter()
        .any(|v| {
            v.target_kind == target_kind
                && v.document_id == document_id
                && v.revision == target_revision
                && v.payment_id == payment_id
        });
        if !valid {
            return Err(Error::Conflict);
        }
        let created = payment_id.is_none();
        let payment = match payment_id {
            Some(p) => p,
            None => {
                crate::treasury_documents::pay(
                    tx,
                    actor,
                    company,
                    &target_kind,
                    &document_id,
                    target_revision,
                    allocated,
                    m.data.booked_on,
                    "bank_transfer",
                    &m.number,
                )
                .await?
            }
        };
        sqlx::query("INSERT INTO bank_match(company_id,movement_id,advance_id,credit_id,payment_id,created_payment,actor_id,amount) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(c).bind(id(&m.id)?).bind(if target_kind=="advance"{Some(id(&document_id)?)}else{None}).bind(if target_kind=="credit"{Some(id(&document_id)?)}else{None}).bind(payment).bind(created).bind(a).bind(allocated).execute(&mut **tx).await.map_err(err)?;
        bump(tx, c, id(&m.id)?).await?;
        return movement(tx, c, id(&m.id)?).await;
    }
    // Lock the selected document before re-evaluating its amount/revision.
    let column = match target_kind.as_str() {
        "invoice" => "invoice_draft",
        "received" => "received_invoice",
        _ => return Err(invalid("target_kind")),
    };
    sqlx::query(&format!(
        "SELECT id FROM {column} WHERE company_id=$1 AND id=$2 FOR UPDATE"
    ))
    .bind(c)
    .bind(id(&document_id)?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(err)?
    .ok_or(Error::NotFound)?;
    // Exact document filter prevents a candidate outside the first search page.
    let number: String = sqlx::query_scalar(&format!(
        "SELECT coalesce(number,'') FROM {column} WHERE company_id=$1 AND id=$2"
    ))
    .bind(c)
    .bind(id(&document_id)?)
    .fetch_one(&mut **tx)
    .await
    .map_err(err)?;
    let valid = candidates(tx, c, &m, &number, Some(id(&document_id)?), false)
        .await?
        .into_iter()
        .any(|v| {
            v.target_kind == target_kind
                && v.document_id == document_id
                && v.revision == target_revision
                && v.payment_id == payment_id
        });
    if !valid {
        return Err(Error::Conflict);
    }
    let created = payment_id.is_none();
    let payment_id = if let Some(p) = payment_id {
        p
    } else {
        let payment = PaymentInput {
            amount: money(amount(&m.data.amount).ok_or(Error::Unavailable)?.abs()),
            paid_on: m.data.booked_on,
            method: "bank_transfer".into(),
            note: m.number.clone(),
        };
        if target_kind == "invoice" {
            let before = crate::payment::load(tx, c, id(&document_id)?).await?;
            let value = crate::payment::register_in_tx(
                tx,
                actor,
                company,
                &document_id,
                payment::Register {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    revision: target_revision,
                    payment,
                },
            )
            .await?;
            value
                .items
                .iter()
                .find(|p| !before.items.iter().any(|old| old.id == p.id))
                .ok_or(Error::Unavailable)?
                .id
                .clone()
        } else {
            let key = uuid::Uuid::new_v4().to_string();
            crate::received::command_in_tx(
                tx,
                actor,
                company,
                &document_id,
                received::Command {
                    request_id: key.clone(),
                    revision: target_revision,
                    action: received::Action::Pay { payment },
                },
            )
            .await?;
            key
        }
    };
    sqlx::query("INSERT INTO bank_match(company_id,movement_id,invoice_id,received_id,payment_id,created_payment,actor_id,amount) VALUES($1,$2,$3,$4,$5,$6,$7,$8)")
              .bind(c).bind(id(&m.id)?).bind(if target_kind=="invoice"{Some(id(&document_id)?)}else{None}).bind(if target_kind=="received"{Some(id(&document_id)?)}else{None})
              .bind(payment_id).bind(created).bind(a).bind(allocated).execute(&mut **tx).await.map_err(err)?;
    bump(tx, c, id(&m.id)?).await?;
    movement(tx, c, id(&m.id)?).await
}

pub(super) async fn batch_preview(
    tx: &mut Transaction<'_, Postgres>,
    c: i64,
    input: BatchInput,
) -> Result<BatchPreview> {
    if input.movement_ids.is_empty() || input.movement_ids.len() > 25 {
        return Err(invalid("movement_ids"));
    }
    let mut ids: Vec<i64> = input
        .movement_ids
        .iter()
        .map(|s| id(s))
        .collect::<Result<_>>()?;
    ids.sort_unstable();
    ids.dedup();
    if ids.len() != input.movement_ids.len() {
        return Err(invalid("movement_ids"));
    }
    let mut items = vec![];
    for id in ids {
        let m = movement(tx, c, id).await?;
        let account = get_account(tx, c, &m.account_id).await?;
        let mut candidate = None;
        let status = if m.matches.iter().any(|v| v.detached_at.is_none()) {
            "matched"
        } else if account.archived {
            "archived"
        } else if m.data.variable_symbol.is_empty() {
            "no_symbol"
        } else {
            let choices = match suggestions(tx, c, &m).await {
                Ok(v) => v,
                Err(Error::Invalid(fields)) if fields.iter().any(|f| f == "suggestions_limit") => {
                    items.push(BatchItem {
                        movement: m,
                        candidate: None,
                        status: "too_many".into(),
                    });
                    continue;
                }
                Err(e) => return Err(e),
            };
            if choices.is_empty() {
                "no_match"
            } else if choices.len() != 1 {
                "ambiguous"
            } else {
                let chosen = choices.into_iter().next().ok_or(Error::Unavailable)?;
                if chosen.payment_id.is_none()
                    && amount(&chosen.outstanding) != amount(&m.data.amount).map(|v| v.abs())
                {
                    "partial"
                } else {
                    // Look beyond the selected page, including other accounts in this profile.
                    let competing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bank_movement m WHERE company_id=$1 AND id<>$2 AND amount=$3 AND coalesce(data->>'variable_symbol','')<>'' AND ltrim(data->>'variable_symbol','0')=$4 AND NOT EXISTS(SELECT 1 FROM bank_match b WHERE b.company_id=m.company_id AND b.movement_id=m.id AND b.detached_at IS NULL))")
                        .bind(c).bind(id).bind(amount(&m.data.amount).ok_or(Error::Unavailable)?).bind(m.data.variable_symbol.trim_start_matches('0')).fetch_one(&mut **tx).await.map_err(err)?;
                    if competing {
                        "competing_movement"
                    } else {
                        candidate = Some(chosen);
                        "ready"
                    }
                }
            }
        };
        items.push(BatchItem {
            movement: m,
            candidate,
            status: status.into(),
        });
    }
    let mut targets = HashMap::new();
    for i in &items {
        if let Some(c) = &i.candidate {
            *targets
                .entry((c.target_kind.clone(), c.document_id.clone()))
                .or_insert(0) += 1;
        }
    }
    for i in &mut items {
        if i.candidate.as_ref().is_some_and(|c| {
            targets
                .get(&(c.target_kind.clone(), c.document_id.clone()))
                .is_some_and(|n| *n > 1)
        }) {
            i.status = "shared_target".into();
            i.candidate = None;
        }
    }
    let ready_count = items.iter().filter(|i| i.status == "ready").count();
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(c, &items)).map_err(err)?)
    );
    Ok(BatchPreview {
        items,
        ready_count,
        fingerprint,
    })
}

pub(super) async fn apply_batch(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    a: i64,
    c: i64,
    report: BatchPreview,
) -> Result<Vec<Movement>> {
    let mut matched = vec![];
    for item in report.items.into_iter().filter(|i| i.status == "ready") {
        let candidate = item.candidate.ok_or(Error::Unavailable)?;
        matched.push(
            match_one(
                tx,
                actor,
                company,
                a,
                c,
                Action::Match {
                    movement_id: item.movement.id,
                    revision: item.movement.revision,
                    target_kind: candidate.target_kind,
                    document_id: candidate.document_id,
                    target_revision: candidate.revision,
                    payment_id: candidate.payment_id,
                },
            )
            .await?,
        );
    }

    Ok(matched)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn split(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    a: i64,
    c: i64,
    movement_id: &str,
    revision: i64,
    allocations: Vec<Allocation>,
) -> Result<Movement> {
    let mut m = movement(tx, c, id(movement_id)?).await?;
    if m.revision != revision {
        return Err(Error::Conflict);
    }
    if allocations.is_empty() || allocations.len() > 25 {
        return Err(invalid("split_items"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut total = rust_decimal::Decimal::ZERO;
    for p in &allocations {
        if !seen.insert((p.target_kind.clone(), id(&p.document_id)?)) {
            return Err(invalid("split_duplicate"));
        }
        let value = amount(&p.amount)
            .filter(|v| *v > rust_decimal::Decimal::ZERO)
            .ok_or_else(|| invalid("split_amount"))?;
        total += value;
    }
    if total > amount(&m.remaining).ok_or(Error::Unavailable)? {
        return Err(invalid("split_amount"));
    }
    for p in allocations {
        let part = amount(&p.amount).ok_or_else(|| invalid("split_amount"))?;
        m = match_part(
            tx,
            actor,
            company,
            a,
            c,
            Action::Match {
                movement_id: m.id,
                revision: m.revision,
                target_kind: p.target_kind,
                document_id: p.document_id,
                target_revision: p.target_revision,
                payment_id: p.payment_id,
            },
            Some(part),
        )
        .await?;
    }
    Ok(m)
}

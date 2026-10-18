use super::*;
pub(super) async fn match_one(
    tx: &mut Transaction<'_, Postgres>,
    actor: &str,
    company: &str,
    a: i64,
    c: i64,
    action: Action,
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
    let m = movement(tx, c, id(&movement_id)?).await?;
    if m.retail_posting_id.is_some() || m.tax.is_some() {
        return Err(invalid("cash_tax_match"));
    }
    if m.cancelled || m.revision != revision || m.matches.iter().any(|m| m.detached_at.is_none()) {
        return Err(Error::Conflict);
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
            amount(&m.data.amount).ok_or(Error::Unavailable)?.abs(),
            m.data.booked_on,
            "cash",
            "CZK",
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
                    amount(&m.data.amount).ok_or(Error::Unavailable)?.abs(),
                    m.data.booked_on,
                    "cash",
                    &m.number,
                )
                .await?
            }
        };
        sqlx::query("INSERT INTO cash_match(company_id,movement_id,advance_id,credit_id,payment_id,created_payment,actor_id) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(c).bind(id(&m.id)?).bind(if target_kind=="advance"{Some(id(&document_id)?)}else{None}).bind(if target_kind=="credit"{Some(id(&document_id)?)}else{None}).bind(payment).bind(created).bind(a).execute(&mut **tx).await.map_err(err)?;
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
    let valid = candidates(tx, c, &m, &number, Some(id(&document_id)?))
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
            method: "cash".into(),
            note: m.number.clone(),
        };
        if target_kind == "invoice" {
            let before = crate::payment::load(tx, c, id(&document_id)?).await?;
            let value = crate::payment::register_without_cash_in_tx(
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
            crate::received::command_without_cash_in_tx(
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
    sqlx::query("INSERT INTO cash_match(company_id,movement_id,invoice_id,received_id,payment_id,created_payment,actor_id) VALUES($1,$2,$3,$4,$5,$6,$7)")
              .bind(c).bind(id(&m.id)?).bind(if target_kind=="invoice"{Some(id(&document_id)?)}else{None}).bind(if target_kind=="received"{Some(id(&document_id)?)}else{None})
              .bind(payment_id).bind(created).bind(a).execute(&mut **tx).await.map_err(err)?;
    bump(tx, c, id(&m.id)?).await?;
    movement(tx, c, id(&m.id)?).await
}

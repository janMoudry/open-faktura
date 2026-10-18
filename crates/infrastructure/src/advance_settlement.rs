use crate::draft::{audit as draft_audit, authorize, map as draft_map};
use async_trait::async_trait;
use faktura_application::{
    advance_settlement::*,
    company::{CompanyError as Error, Result},
    draft::Draft,
};
use faktura_domain::{
    advance_settlement::{same_party, Settlement, Source, TaxDeduction},
    invoice::{Calculation, PriceMode, Rounding, Totals},
};
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::str::FromStr;
#[derive(Clone)]
pub struct PgSettlement(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(v: &str) -> Result<i64> {
    v.parse().map_err(|_| Error::NotFound)
}
fn dec(v: &str) -> Result<Decimal> {
    Decimal::from_str(v).map_err(err)
}
fn money(mut v: Decimal) -> String {
    v.rescale(2);
    v.to_string()
}
pub(crate) async fn ensure_free(tx: &mut Transaction<'_, Postgres>, c: i64, d: i64) -> Result<()> {
    let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM advance_settlement_link WHERE company_id=$1 AND advance_id=$2)").bind(c).bind(d).fetch_one(&mut **tx).await.map_err(err)?;
    if used {
        Err(Error::Conflict)
    } else {
        Ok(())
    }
}
#[async_trait]
impl SettlementStore for PgSettlement {
    async fn configure(&self, a: &str, c: &str, cmd: Configure) -> Result<Draft> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (a, c) = authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let input = serde_json::to_value(&cmd).map_err(err)?;
        if let Some(r)=sqlx::query("SELECT * FROM advance_settlement_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&cmd.request_id).fetch_optional(&mut *tx).await.map_err(err)?{
 if r.get::<i64,_>("actor_id")!=a||r.get::<serde_json::Value,_>("command")!=input{return Err(Error::Conflict)}
 return draft_map(sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2").bind(c).bind(r.get::<i64,_>("invoice_id")).fetch_one(&mut *tx).await.map_err(err)?);
 }
        let old = if let Some(ref i) = cmd.invoice_id {
            Some(draft_map(
                sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2 FOR UPDATE")
                    .bind(c)
                    .bind(id(i)?)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(err)?
                    .ok_or(Error::NotFound)?,
            )?)
        } else {
            None
        };
        if old
            .as_ref()
            .is_some_and(|v| v.number.is_some() || Some(v.revision) != cmd.revision)
        {
            return Err(Error::Conflict);
        }
        let today: chrono::NaiveDate =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Europe/Prague')::date")
                .fetch_one(&mut *tx)
                .await
                .map_err(err)?;
        let mut sources = Vec::new();
        let mut documents = Vec::new();
        let mut credited = Decimal::ZERO;
        let mut rounding = Decimal::ZERO;
        let mut minimum_date = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
        let mut selections: Vec<_> = cmd.advances.iter().collect();
        selections.sort_by_key(|s| s.id.parse::<i64>().unwrap_or(0));
        // Lock all involved advances in ID order, including ones being released.
        let mut locks: Vec<i64> = selections
            .iter()
            .map(|s| id(&s.id))
            .collect::<Result<_>>()?;
        if let Some(v) = &old {
            locks.extend(sqlx::query_scalar::<_,i64>("SELECT advance_id FROM advance_settlement_link WHERE company_id=$1 AND invoice_id=$2").bind(c).bind(id(&v.id)?).fetch_all(&mut *tx).await.map_err(err)?);
        }
        locks.sort_unstable();
        locks.dedup();
        for lock in locks {
            sqlx::query("SELECT id FROM sales_document WHERE company_id=$1 AND id=$2 FOR UPDATE")
                .bind(c)
                .bind(lock)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?;
        }
        for selected in selections {
            let v = crate::sales::map(
                sqlx::query(
                    "SELECT * FROM sales_document WHERE company_id=$1 AND id=$2 AND kind='advance'",
                )
                .bind(c)
                .bind(id(&selected.id)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
            )?;
            if v.archived
                || v.revision != selected.revision
                || v.invoice_id
                    .as_ref()
                    .is_some_and(|i| Some(i) != cmd.invoice_id.as_ref())
            {
                return Err(Error::Conflict);
            }
            let t = v.totals.as_ref().ok_or(Error::Conflict)?;
            let paid = dec(v.advance_paid.as_deref().unwrap_or("0"))?;
            if paid <= Decimal::ZERO || paid != dec(&t.payable)? {
                return Err(Error::Invalid(vec!["advance_not_fully_paid".into()]));
            }
            let party = v.data.invoice.customer.as_ref().ok_or(Error::Conflict)?;
            if let Some(first) = documents.first() {
                let first: &faktura_application::sales::Document = first;
                if first.data.invoice.calculation.currency != v.data.invoice.calculation.currency
                    || first.data.invoice.calculation.exchange_rate
                        != v.data.invoice.calculation.exchange_rate
                    || !same_party(first.data.invoice.customer.as_ref().unwrap(), party)
                    || !same_party(&first.data.invoice.supplier, &v.data.invoice.supplier)
                    || first.data.invoice.calculation.tax_mode
                        != v.data.invoice.calculation.tax_mode
                {
                    return Err(Error::Invalid(vec!["advance_identity_or_date".into()]));
                }
            }
            let payments=sqlx::query("SELECT id,paid_on FROM advance_payment WHERE company_id=$1 AND document_id=$2 AND reversed_at IS NULL ORDER BY id").bind(c).bind(id(&v.id)?).fetch_all(&mut *tx).await.map_err(err)?;
            let mut payment_ids = Vec::new();
            for p in payments {
                payment_ids.push(p.get::<i64, _>("id").to_string());
                minimum_date = minimum_date.max(p.get("paid_on"));
            }
            let receipts=sqlx::query("SELECT snapshot FROM tax_receipt WHERE company_id=$1 AND advance_id=$2 ORDER BY id").bind(c).bind(id(&v.id)?).fetch_all(&mut *tx).await.map_err(err)?;
            let mut taxes = Vec::new();
            for r in receipts {
                let t: faktura_application::tax_receipt::Document =
                    serde_json::from_value(r.get("snapshot")).map_err(err)?;
                minimum_date = minimum_date.max(t.data.issued_on.ok_or(Error::Conflict)?);
                taxes.push(TaxDeduction {
                    id: t.id,
                    number: t.number,
                    totals: t.totals,
                });
            }
            credited += paid;
            rounding += dec(&t.rounding)?;
            sources.push(Source {
                id: v.id.clone(),
                number: v.number.clone().unwrap_or_default(),
                paid: money(paid),
                payment_ids,
                tax_receipts: taxes,
            });
            documents.push(v);
        }
        let mut data = if let Some(v) = &old {
            v.data.clone()
        } else {
            let first = &documents[0];
            let mut data = first.data.invoice.clone();
            data.issued_on = Some(today);
            data.due_on = Some(today + chrono::Duration::days(first.data.maturity_days));
            data.tax_on = if data.calculation.tax_mode == faktura_domain::invoice::TaxMode::VatPayer
            {
                Some(today)
            } else {
                None
            };
            data.payment.variable_symbol = String::new();
            let preserve = documents.iter().all(|v| {
                v.data.invoice.calculation.price_mode == first.data.invoice.calculation.price_mode
                    && v.data.invoice.calculation.discount_percent
                        == first.data.invoice.calculation.discount_percent
                    && v.data.invoice.calculation.rounding == Rounding::None
            });
            data.calculation.price_mode = PriceMode::Gross;
            data.calculation.discount_percent = "0".into();
            data.calculation.rounding = Rounding::None;
            data.calculation.lines.clear();
            for v in &documents {
                for (index, line) in v.data.invoice.calculation.lines.iter().enumerate() {
                    let mut line = line.clone();
                    line.id = format!("advance-{}-{index}", v.id);
                    if !preserve {
                        line.quantity = "1".into();
                        line.unit = String::new();
                        line.unit_price = v.totals.as_ref().unwrap().lines[index]
                            .amounts
                            .gross
                            .clone();
                        line.discount_percent = "0".into();
                    }
                    data.calculation.lines.push(line);
                }
            }
            if preserve {
                data.calculation.price_mode = first.data.invoice.calculation.price_mode;
                data.calculation.discount_percent =
                    first.data.invoice.calculation.discount_percent.clone();
            }
            data
        };
        if data.calculation.lines.len() > 500 {
            return Err(Error::Invalid(vec!["lines".into()]));
        }
        let advances = if sources.is_empty() {
            None
        } else {
            Some(Settlement {
                currency: documents[0].data.invoice.calculation.currency.clone(),
                exchange_rate: documents[0].data.invoice.calculation.exchange_rate.clone(),
                version: "of-settlement-v1".into(),
                customer: documents[0].data.invoice.customer.clone().unwrap(),
                supplier: documents[0].data.invoice.supplier.clone(),
                tax_mode: documents[0].data.invoice.calculation.tax_mode,
                sources,
                credited: money(credited),
                rounding_adjustment: if old.is_some() {
                    old.as_ref()
                        .and_then(|v| v.advances.as_ref())
                        .map(|v| v.rounding_adjustment.clone())
                        .unwrap_or_else(|| "0.00".into())
                } else {
                    money(rounding)
                },
                minimum_date,
            })
        };
        let mut totals =
            faktura_domain::invoice::calculate(&data.calculation).map_err(Error::Invalid)?;
        if let Some(s) = &advances {
            faktura_domain::advance_settlement::validate(&data, s).map_err(Error::Invalid)?;
            totals =
                faktura_domain::advance_settlement::apply(totals, s).map_err(Error::Invalid)?;
        }
        crate::draft::check_contact(&mut tx, c, data.contact_id.as_deref()).await?;
        let value = if let Some(v) = &old {
            draft_map(sqlx::query("UPDATE invoice_draft SET advances=$3,totals=$4,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(&v.id)?).bind(serde_json::to_value(&advances).map_err(err)?).bind(serde_json::to_value(totals).map_err(err)?).fetch_one(&mut *tx).await.map_err(err)?)?
        } else {
            if data.note.is_empty() {
                data.note = if data.language == "en" {
                    "Advance settlement"
                } else {
                    "Vyúčtování záloh"
                }
                .into();
            }
            let json = serde_json::to_value(&data).map_err(err)?;
            draft_map(sqlx::query("INSERT INTO invoice_draft(company_id,creator_id,request_id,initial_data,data,totals,contact_id,advances) VALUES($1,$2,$3::text::uuid,$4,$4,$5,$6,$7) RETURNING *").bind(c).bind(a).bind(&cmd.request_id).bind(json).bind(serde_json::to_value(totals).map_err(err)?).bind(data.contact_id.as_deref().map(id).transpose()?).bind(serde_json::to_value(&advances).map_err(err)?).fetch_one(&mut *tx).await.map_err(|e|if e.as_database_error().is_some_and(|e|e.is_unique_violation()){Error::Conflict}else{err(e)})?)?
        };
        if let Some(v) = old {
            let released=sqlx::query("UPDATE sales_document SET invoice_id=NULL,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id IN(SELECT advance_id FROM advance_settlement_link WHERE company_id=$1 AND invoice_id=$2) RETURNING *").bind(c).bind(id(&v.id)?).fetch_all(&mut *tx).await.map_err(err)?;
            for row in released {
                crate::sales::audit(&mut tx, a, &crate::sales::map(row)?, "settlement_release")
                    .await?;
            }

            sqlx::query(
                "DELETE FROM advance_settlement_link WHERE company_id=$1 AND invoice_id=$2",
            )
            .bind(c)
            .bind(id(&v.id)?)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        }
        for selected in &cmd.advances {
            sqlx::query("INSERT INTO advance_settlement_link(company_id,advance_id,invoice_id) VALUES($1,$2,$3)").bind(c).bind(id(&selected.id)?).bind(id(&value.id)?).execute(&mut *tx).await.map_err(|e|if e.as_database_error().is_some_and(|e|e.is_unique_violation()){Error::Conflict}else{err(e)})?;
            let r=sqlx::query("UPDATE sales_document SET invoice_id=$3,revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2 RETURNING *").bind(c).bind(id(&selected.id)?).bind(id(&value.id)?).fetch_one(&mut *tx).await.map_err(err)?;
            crate::sales::audit(&mut tx, a, &crate::sales::map(r)?, "settlement_reserve").await?;
        }
        draft_audit(
            &mut tx,
            a,
            &value,
            if cmd.invoice_id.is_some() {
                "update"
            } else {
                "create"
            },
        )
        .await?;
        sqlx::query("INSERT INTO advance_settlement_command(company_id,request_id,actor_id,command,invoice_id) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(&cmd.request_id).bind(a).bind(input).bind(id(&value.id)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(value)
    }
    async fn candidates(&self, a: &str, c: &str, q: Search) -> Result<Candidates> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let page = q.page.unwrap_or(1);
        let filter="company_id=$1 AND kind='advance' AND NOT archived AND invoice_id IS NULL AND advance_paid>0 AND advance_paid=(advance_totals->>'payable')::numeric AND strpos(lower(concat_ws(' ',number,data->>'name',data#>>'{invoice,customer,name}')),lower($2))>0";
        let total = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM sales_document WHERE {filter}"
        ))
        .bind(c)
        .bind(&q.query)
        .fetch_one(&mut *tx)
        .await
        .map_err(err)?;
        let items = sqlx::query(&format!(
            "SELECT * FROM sales_document WHERE {filter} ORDER BY id DESC LIMIT 50 OFFSET $3"
        ))
        .bind(c)
        .bind(&q.query)
        .bind((page - 1) * 50)
        .fetch_all(&mut *tx)
        .await
        .map_err(err)?
        .into_iter()
        .map(crate::sales::map)
        .collect::<Result<Vec<_>>>()?;
        Ok(Candidates { items, total, page })
    }
    async fn preview(&self, a: &str, c: &str, i: &str, calculation: Calculation) -> Result<Totals> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = authorize(&mut tx, a, c, false).await?;
        let v = draft_map(
            sqlx::query("SELECT * FROM invoice_draft WHERE company_id=$1 AND id=$2")
                .bind(c)
                .bind(id(i)?)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?
                .ok_or(Error::NotFound)?,
        )?;
        let totals = faktura_domain::invoice::calculate(&calculation).map_err(Error::Invalid)?;
        if let Some(s) = v.advances {
            if calculation.tax_mode != s.tax_mode
                || calculation.currency != s.currency
                || calculation.exchange_rate != s.exchange_rate
            {
                return Err(Error::Invalid(vec!["tax_mode".into()]));
            }
            faktura_domain::advance_settlement::apply(totals, &s).map_err(Error::Invalid)
        } else {
            Ok(totals)
        }
    }
}

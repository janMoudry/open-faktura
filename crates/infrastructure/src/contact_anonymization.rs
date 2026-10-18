use async_trait::async_trait;
use faktura_application::{contact::Contact, contact_anonymization::*};
use faktura_domain::contact::{Address, ContactData};
use sqlx::{PgPool, Row};
#[derive(Clone)]
pub struct PgContactAnonymization(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn id(value: &str) -> Result<i64> {
    value.parse().map_err(|_| Error::NotFound)
}
#[async_trait]
impl AnonymizationStore for PgContactAnonymization {
    async fn execute(
        &self,
        actor: &str,
        company: &str,
        contact: &str,
        command: Command,
    ) -> Result<Contact> {
        let (actor, company, contact) = (id(actor)?, id(company)?, id(contact)?);
        let mut tx = self.0.begin().await.map_err(err)?;
        // Serialize role changes/removal and contact writers before replay/row access.
        let role: String=sqlx::query_scalar("SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR UPDATE OF c,m")
            .bind(company).bind(actor).fetch_optional(&mut *tx).await.map_err(err)?.ok_or(Error::NotFound)?;
        if role != "owner" {
            return Err(Error::Forbidden);
        }
        if !command.confirm
            || command.revision < 1
            || uuid::Uuid::parse_str(&command.request_id).is_err()
        {
            return Err(Error::Invalid(vec!["confirmation".into()]));
        }
        let encoded = serde_json::to_value(&command).map_err(err)?;
        if let Some(row)=sqlx::query("SELECT actor_id,contact_id,command,result FROM contact_anonymization_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(company).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)? {
            if row.get::<i64,_>("actor_id")!=actor || row.get::<i64,_>("contact_id")!=contact || row.get::<serde_json::Value,_>("command")!=encoded { return Err(Error::Conflict); }
            let result=serde_json::from_value(row.get("result")).map_err(err)?;
            tx.commit().await.map_err(err)?;
            return Ok(result);
        }
        let row = sqlx::query(
            "SELECT revision,anonymized_at FROM contact WHERE company_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(company)
        .bind(contact)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?
        .ok_or(Error::NotFound)?;
        if row.get::<i64, _>("revision") != command.revision
            || row
                .get::<Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>, _>(
                    "anonymized_at",
                )
                .is_some()
        {
            return Err(Error::Conflict);
        }
        let data = ContactData {
            name: format!("— #{contact}"),
            billing_address: Address {
                country: String::new(),
                ..Default::default()
            },
            ..Default::default()
        };
        let json = serde_json::to_value(&data).map_err(err)?;
        sqlx::query("UPDATE contact SET data=$3,initial_data=$3,archived=true,anonymized_at=now(),revision=revision+1,updated_at=now() WHERE company_id=$1 AND id=$2").bind(company).bind(contact).bind(&json).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("UPDATE contact_audit SET data=$3 WHERE company_id=$1 AND contact_id=$2")
            .bind(company)
            .bind(contact)
            .bind(&json)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        sqlx::query("INSERT INTO contact_audit(company_id,contact_id,revision,actor_id,action,data,archived) VALUES($1,$2,$3,$4,'anonymize',$5,true)").bind(company).bind(contact).bind(command.revision+1).bind(actor).bind(&json).execute(&mut *tx).await.map_err(err)?;
        let result = Contact {
            id: contact.to_string(),
            company_id: company.to_string(),
            revision: command.revision + 1,
            archived: true,
            anonymized: true,
            data,
        };
        sqlx::query("INSERT INTO contact_anonymization_command(company_id,request_id,contact_id,actor_id,command,result) VALUES($1,$2::text::uuid,$3,$4,$5,$6)").bind(company).bind(&command.request_id).bind(contact).bind(actor).bind(encoded).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

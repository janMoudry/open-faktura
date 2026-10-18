use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    taxpayer_filing_profile::*,
};
use sqlx::{PgPool, Row};
pub struct PgTaxpayerFilingProfile(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
#[async_trait]
impl TaxpayerFilingProfileStore for PgTaxpayerFilingProfile {
    async fn get(&self, a: &str, c: &str) -> Result<Profile> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, a, c, false).await?;
        let row =
            sqlx::query("SELECT revision,data FROM taxpayer_filing_profile WHERE company_id=$1")
                .bind(c)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        let value = match row {
            Some(r) => Profile {
                revision: r.get("revision"),
                data: Some(serde_json::from_value(r.get("data")).map_err(err)?),
            },
            None => Profile {
                revision: 0,
                data: None,
            },
        };
        tx.commit().await.map_err(err)?;
        Ok(value)
    }
    async fn set(&self, a: &str, c: &str, input: Command) -> Result<Profile> {
        validate(&input)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let (actor, c) = crate::draft::authorize(&mut tx, a, c, true).await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        let request = uuid::Uuid::parse_str(&input.request_id).map_err(err)?;
        let value = serde_json::to_value(&input).map_err(err)?;
        if let Some(row)=sqlx::query("SELECT actor_id,input,result FROM taxpayer_filing_profile_command WHERE company_id=$1 AND request_id=$2").bind(c).bind(request).fetch_optional(&mut *tx).await.map_err(err)? {
   if row.get::<i64,_>("actor_id")!=actor || row.get::<serde_json::Value,_>("input")!=value {return Err(Error::Conflict)}
   return serde_json::from_value(row.get("result")).map_err(err)
  }
        let revision: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM taxpayer_filing_profile WHERE company_id=$1")
                .bind(c)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        if revision.unwrap_or(0) != input.revision {
            return Err(Error::Conflict);
        }
        let data = serde_json::to_value(&input.data).map_err(err)?;
        let revision:i64=sqlx::query_scalar("INSERT INTO taxpayer_filing_profile(company_id,data,updated_by) VALUES($1,$2,$3) ON CONFLICT(company_id) DO UPDATE SET data=excluded.data,revision=taxpayer_filing_profile.revision+1,updated_by=excluded.updated_by,updated_at=now() RETURNING revision").bind(c).bind(&data).bind(actor).fetch_one(&mut *tx).await.map_err(err)?;
        let result = Profile {
            revision,
            data: Some(input.data),
        };
        sqlx::query("INSERT INTO taxpayer_filing_profile_audit(company_id,actor_id,revision,data) VALUES($1,$2,$3,$4)").bind(c).bind(actor).bind(revision).bind(data).execute(&mut *tx).await.map_err(err)?;
        sqlx::query("INSERT INTO taxpayer_filing_profile_command(company_id,request_id,actor_id,input,result) VALUES($1,$2,$3,$4,$5)").bind(c).bind(request).bind(actor).bind(value).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

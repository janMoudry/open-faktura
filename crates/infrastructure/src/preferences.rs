use async_trait::async_trait;
use faktura_application::preferences::*;
use sqlx::{PgPool, Row};
pub struct PgPreferences(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn map(r: sqlx::postgres::PgRow) -> Result<Preferences> {
    Ok(Preferences {
        revision: r.get("revision"),
        theme: serde_json::from_value(serde_json::Value::String(r.get("theme"))).map_err(err)?,
    })
}
#[async_trait]
impl PreferenceStore for PgPreferences {
    async fn get(&self, a: &str) -> Result<Preferences> {
        let a: i64 = a.parse().map_err(|_| Error::NotFound)?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM app_user WHERE id=$1)")
            .bind(a)
            .fetch_one(&self.0)
            .await
            .map_err(err)?;
        if !exists {
            return Err(Error::NotFound);
        }
        match sqlx::query("SELECT * FROM user_preference WHERE user_id=$1")
            .bind(a)
            .fetch_optional(&self.0)
            .await
            .map_err(err)?
        {
            Some(r) => map(r),
            None => Ok(Preferences {
                revision: 0,
                theme: Theme::System,
            }),
        }
    }
    async fn set(&self, a: &str, input: Preferences) -> Result<Preferences> {
        let a: i64 = a.parse().map_err(|_| Error::NotFound)?;
        if input.revision < 0 || input.revision > 1_000_000_000_000 {
            return Err(Error::Invalid(vec!["revision".into()]));
        }
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query_scalar::<_, i64>("SELECT id FROM app_user WHERE id=$1 FOR UPDATE")
            .bind(a)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
        let current = sqlx::query("SELECT * FROM user_preference WHERE user_id=$1")
            .bind(a)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?;
        if current
            .as_ref()
            .map(|r| r.get::<i64, _>("revision"))
            .unwrap_or(0)
            != input.revision
        {
            // A retry of the immediately applied same value is safe.
            if let Some(r) = current {
                let value = map(r)?;
                if value.revision == input.revision + 1 && value.theme == input.theme {
                    return Ok(value);
                }
            }
            return Err(Error::Conflict);
        }
        let row=sqlx::query("INSERT INTO user_preference(user_id,theme) VALUES($1,$2) ON CONFLICT(user_id) DO UPDATE SET theme=excluded.theme,revision=user_preference.revision+1,updated_at=now() RETURNING *").bind(a).bind(input.theme.as_str()).fetch_one(&mut *tx).await.map_err(err)?;
        let result = map(row)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

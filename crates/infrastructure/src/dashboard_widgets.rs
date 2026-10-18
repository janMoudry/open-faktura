use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    dashboard_widgets::*,
};
use sqlx::{PgPool, Row};
pub struct PgDashboardWidgets(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
fn map(r: sqlx::postgres::PgRow) -> Result<Layout> {
    Ok(Layout {
        revision: r.get("revision"),
        widgets: serde_json::from_value(r.get("widgets")).map_err(err)?,
    })
}
#[async_trait]
impl DashboardWidgetStore for PgDashboardWidgets {
    async fn get(&self, a: &str) -> Result<Layout> {
        let a: i64 = a.parse().map_err(|_| Error::NotFound)?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM app_user WHERE id=$1)")
            .bind(a)
            .fetch_one(&self.0)
            .await
            .map_err(err)?;
        if !exists {
            return Err(Error::NotFound);
        }
        match sqlx::query("SELECT revision,widgets FROM user_dashboard_layout WHERE user_id=$1")
            .bind(a)
            .fetch_optional(&self.0)
            .await
            .map_err(err)?
        {
            Some(r) => map(r),
            None => Ok(Layout::default()),
        }
    }
    async fn set(&self, a: &str, input: Layout) -> Result<Layout> {
        validate(&input)?;
        let a: i64 = a.parse().map_err(|_| Error::NotFound)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        sqlx::query_scalar::<_, i64>("SELECT id FROM app_user WHERE id=$1 FOR UPDATE")
            .bind(a)
            .fetch_optional(&mut *tx)
            .await
            .map_err(err)?
            .ok_or(Error::NotFound)?;
        let old =
            sqlx::query("SELECT revision,widgets FROM user_dashboard_layout WHERE user_id=$1")
                .bind(a)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        let old = old.map(map).transpose()?.unwrap_or_default();
        if old.revision != input.revision {
            if old.revision == input.revision + 1 && old.widgets == input.widgets {
                return Ok(old);
            }
            return Err(Error::Conflict);
        }
        let row=sqlx::query("INSERT INTO user_dashboard_layout(user_id,widgets) VALUES($1,$2) ON CONFLICT(user_id) DO UPDATE SET widgets=excluded.widgets,revision=user_dashboard_layout.revision+1,updated_at=now() RETURNING revision,widgets").bind(a).bind(serde_json::to_value(&input.widgets).map_err(err)?).fetch_one(&mut *tx).await.map_err(err)?;
        let result = map(row)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}

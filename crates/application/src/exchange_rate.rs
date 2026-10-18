//! Read-only rate proposal; document commands persist the selected snapshot.
use crate::company::{CompanyError as Error, CompanyStore, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use faktura_domain::exchange::ExchangeRate;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rate {
    pub currency: String,
    pub requested_on: NaiveDate,
    pub published_on: NaiveDate,
    pub source: String,
    pub exchange_rate: ExchangeRate,
}
#[async_trait]
pub trait RateProvider: Send + Sync {
    async fn lookup(&self, currency: &str, date: NaiveDate) -> Result<Rate>;
}
pub async fn lookup(
    store: &dyn CompanyStore,
    provider: &dyn RateProvider,
    actor: &str,
    company: &str,
    currency: &str,
    date: &str,
) -> Result<Rate> {
    store.get(actor, company).await?;
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| Error::Invalid(vec!["date".into()]))?;
    if currency == "CZK"
        || !faktura_domain::exchange::CURRENCIES.contains(&currency)
        || date < NaiveDate::from_ymd_opt(2002, 1, 1).unwrap()
    {
        return Err(Error::Invalid(vec!["currency_or_date".into()]));
    }
    provider.lookup(currency, date).await
}

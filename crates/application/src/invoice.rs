use crate::company::{CompanyError, CompanyStore, Result};
use faktura_domain::invoice::{self, Calculation, Totals};
/// Preview has no writes or number allocation; all members may read/calculate.
pub async fn calculate(
    store: &dyn CompanyStore,
    actor: &str,
    company: &str,
    input: Calculation,
) -> Result<Totals> {
    store.get(actor, company).await?;
    invoice::calculate(&input).map_err(CompanyError::Invalid)
}

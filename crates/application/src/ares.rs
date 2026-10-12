//! Registry lookup only proposes values; saving still uses contact commands.
use crate::company::{CompanyError as Error, CompanyStore, Result};
use async_trait::async_trait;
use faktura_domain::contact::Address;
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct RegistryContact {
    pub registration_number: String,
    pub name: String,
    pub vat_number: String,
    pub billing_address: Address,
}
#[async_trait]
pub trait Registry: Send + Sync {
    async fn lookup(&self, ico: &str) -> Result<RegistryContact>;
}
pub fn valid_ico(ico: &str) -> bool {
    if ico.len() != 8 || !ico.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let digits: Vec<u32> = ico.bytes().map(|b| u32::from(b - b'0')).collect();
    let sum: u32 = digits[..7]
        .iter()
        .enumerate()
        .map(|(i, d)| d * (8 - i as u32))
        .sum();
    digits[7] == (11 - sum % 11) % 10
}
pub async fn lookup(
    store: &dyn CompanyStore,
    registry: &dyn Registry,
    actor: &str,
    company: &str,
    ico: &str,
) -> Result<RegistryContact> {
    store.get(actor, company).await?;
    if !valid_ico(ico) {
        return Err(Error::Invalid(vec!["registration_number".into()]));
    }
    registry.lookup(ico).await
}
#[cfg(test)]
mod tests {
    #[test]
    fn checksum() {
        assert!(super::valid_ico("00006947"));
        for ico in ["00006948", "6947", "abcdefgh", "00000000"] {
            assert!(!super::valid_ico(ico));
        }
    }
}

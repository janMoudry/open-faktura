use faktura_application::bank_connection::{Error, Result};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
fn err(_: impl std::fmt::Debug) -> Error {
    Error::Unavailable
}
fn key() -> Result<aead::LessSafeKey> {
    let path =
        std::env::var("BANK_ENCRYPTION_KEY_FILE").unwrap_or_else(|_| ".secrets/bank-key".into());
    let raw = std::fs::read_to_string(path).map_err(err)?;
    let raw = raw.trim();
    if raw.len() != 64 {
        return Err(Error::Unavailable);
    }
    let bytes = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(raw.get(i..i + 2).ok_or(Error::Unavailable)?, 16).map_err(err))
        .collect::<Result<Vec<_>>>()?;
    Ok(aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_256_GCM, &bytes).map_err(err)?,
    ))
}
pub(super) fn encrypt(value: &str, context: &str) -> Result<Vec<u8>> {
    let key = key()?;
    let mut nonce = [0; 12];
    SystemRandom::new().fill(&mut nonce).map_err(err)?;
    let mut data = value.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(context.as_bytes()),
        &mut data,
    )
    .map_err(err)?;
    let mut result = nonce.to_vec();
    result.extend(data);
    Ok(result)
}
pub(super) fn decrypt(value: &[u8], context: &str) -> Result<String> {
    if value.len() < 28 {
        return Err(Error::Unavailable);
    }
    let nonce = value[..12].try_into().map_err(err)?;
    let mut data = value[12..].to_vec();
    let key = key()?;
    let plain = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(context.as_bytes()),
            &mut data,
        )
        .map_err(err)?;
    String::from_utf8(plain.to_vec()).map_err(err)
}

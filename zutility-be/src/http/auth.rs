use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

pub fn hash_order_token(secret: &SecretString, token: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
        .map_err(|_| String::from("invalid hmac key"))?;
    mac.update(token.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(STANDARD.encode(digest))
}

pub fn verify_order_token_hash(secret: &SecretString, token: &str, expected_hash: &str) -> bool {
    match hash_order_token(secret, token) {
        Ok(computed) => computed.as_bytes().ct_eq(expected_hash.as_bytes()).into(),
        Err(_) => false,
    }
}

pub fn hash_ip(secret: &SecretString, ip: &str) -> Result<String, String> {
    let normalized = ip.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(String::from("ip cannot be empty"));
    }

    let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
        .map_err(|_| String::from("invalid hmac key"))?;
    mac.update(normalized.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(STANDARD.encode(digest))
}

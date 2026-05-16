use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use base64::{Engine as _, engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}};
use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, Algorithm};
use rand::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

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

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("password hashing failed: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(hash).map_err(|e| format!("invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub email: String,
    pub csrf: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn create_access_jwt(
    user_id: Uuid,
    email: &str,
    csrf_token: &str,
    secret: &SecretString,
    ttl_minutes: i64,
) -> Result<String, String> {
    let now = Utc::now();
    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_owned(),
        csrf: csrf_token.to_owned(),
        exp: now.timestamp() + (ttl_minutes * 60),
        iat: now.timestamp(),
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.expose_secret().as_bytes()),
    )
    .map_err(|e| format!("jwt encoding failed: {e}"))
}

pub fn verify_access_jwt(
    token: &str,
    secret: &SecretString,
) -> Result<AccessTokenClaims, String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let data = jsonwebtoken::decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.expose_secret().as_bytes()),
        &validation,
    )
    .map_err(|e| format!("jwt verification failed: {e}"))?;

    Ok(data.claims)
}

pub fn generate_csrf_token() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random_range(0u8..=255)).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_verification_token() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..48).map(|_| rng.random_range(0u8..=255)).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_verification_token(token: &str) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(b"zutility-email-token")
        .map_err(|_| String::from("invalid hmac key"))?;
    mac.update(token.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(STANDARD.encode(digest))
}

pub fn is_valid_email(email: &str) -> bool {
    if email.is_empty() {
        return false;
    }
    let parts: Vec<&str> = email.rsplitn(2, '@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (domain, local) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    true
}

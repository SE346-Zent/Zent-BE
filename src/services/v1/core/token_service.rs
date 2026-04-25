use jsonwebtoken::{encode, Header, EncodingKey};
use crate::model::jwt_claims::Claims;
use crate::core::errors::AppError;
use base64::{engine::general_purpose::URL_SAFE, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use chrono::Utc;

pub struct TokenBundle {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_token_hash: String,
}

/// Generate a new pair of access and refresh tokens
pub fn generate_token_bundle(
    user_id: &str,
    access_token_ttl_seconds: i64,
    encoding_key: &EncodingKey,
) -> Result<TokenBundle, AppError> {
    let now = Utc::now().timestamp();

    // 1. Generate Access Token (JWT)
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now as usize,
        exp: (now + access_token_ttl_seconds) as usize,
    };

    let access_token = encode(&Header::default(), &claims, encoding_key)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to encode JWT: {}", e)))?;

    // 2. Generate Raw Refresh Token
    let mut refresh_token_bytes = [0u8; 48];
    rand::rng().fill_bytes(&mut refresh_token_bytes);
    let refresh_token = URL_SAFE.encode(refresh_token_bytes);

    // 3. Hash Refresh Token for storage
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    let refresh_token_hash = format!("{:x}", hasher.finalize());

    Ok(TokenBundle {
        access_token,
        refresh_token,
        refresh_token_hash,
    })
}

/// Utility to hash a refresh token (useful for lookups)
pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

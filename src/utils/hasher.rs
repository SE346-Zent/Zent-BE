use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use crate::core::errors::AppError;

/// Hash a plain text password using Argon2.
pub async fn hash_password(password: String) -> Result<String, AppError> {
    let password_bytes = password.into_bytes();
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(&password_bytes, &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to spawn blocking task for hashing")))?
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))
}

/// Verify a plain text password against an Argon2 hash.
pub async fn verify_password(password: String, hashed_password: String) -> Result<bool, AppError> {
    let password_bytes = password.into_bytes();
    tokio::task::spawn_blocking(move || {
        let parsed_hash = match PasswordHash::new(&hashed_password) {
            Ok(h) => h,
            Err(_) => return Ok(false),
        };
        
        Ok(Argon2::default()
            .verify_password(&password_bytes, &parsed_hash)
            .is_ok())
    })
    .await
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to spawn blocking task for verification")))?
    .map_err(|e: anyhow::Error| AppError::Internal(e))
}

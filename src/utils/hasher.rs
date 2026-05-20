use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use crate::core::errors::AppError;

/// Hash a plain text password using the Argon2 hashing algorithm.
///
/// This function is computationally expensive and is executed on a dedicated
/// blocking thread pool to avoid blocking the async runtime.
///
/// # Arguments
/// * `plain_password` - The raw password string to be hashed.
///
/// # Returns
/// A result containing the encoded Argon2 hash string or an `AppError`.
pub async fn hash_password(plain_password: String) -> Result<String, AppError> {
    let password_bytes = plain_password.into_bytes();
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

/// Verify a plain text password against an existing Argon2 hash string.
///
/// # Arguments
/// * `plain_password` - The raw password string provided by the user.
/// * `hashed_password` - The stored Argon2 hash string to verify against.
///
/// # Returns
/// A result containing `true` if the password matches the hash, `false` otherwise, or an `AppError`.
pub async fn verify_password(plain_password: String, hashed_password: String) -> Result<bool, AppError> {
    let password_bytes = plain_password.into_bytes();
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

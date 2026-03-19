use crate::entities::{account_status, role, user};
use crate::model::requests::auth::user_registration_request::UserRegistrationRequest;
use crate::model::responses::auth::register_response::RegisterResponse;
use crate::model::responses::error::AppError;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

pub async fn perform_register(
    db: DatabaseConnection,
    req: UserRegistrationRequest,
) -> Result<RegisterResponse, AppError> {
    // 3. Check if user already exist with email
    let existing_user = user::Entity::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "User with email already exists".to_string(),
        ));
    }

    // 4. Construct User entity object
    // TODO: Implement in-memory lookup table that loads DB Role table for fast, type-safe lookup.
    let customer_role = role::Entity::find()
        .filter(role::Column::Name.eq("Customer"))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading role: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role not found")))?;

    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading account status: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status not found")))?;

    let password_bytes = req.password.into_bytes();
    let hashed_password = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(&password_bytes, &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to spawn blocking task")))?
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    let now = Utc::now();

    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        full_name: Set(req.fullname),
        email: Set(req.email),
        password_hash: Set(hashed_password),
        phone_number: Set(req.phonenumber),
        account_status: Set(pending_status.id), // Using id from pending_status
        role_id: Set(customer_role.id),         // Using id from customer_role
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    // 5. Persist record
    new_user
        .insert(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create user: {:?}", e)))?;

    // 6. Finalise
    // TODO: Add send verification email here once implemented
    // TODO: implement full phone number format validation (format & country code).

    Ok(RegisterResponse::success("User registered successfully"))
}

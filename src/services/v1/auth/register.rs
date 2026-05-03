use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::{
    core::errors::AppError,
    entities::{account_status, roles, users},
    model::{
        requests::auth::user_registration_request::UserRegistrationRequest,
        responses::base::ApiResponse,
    },
    services::v1::core::email_service,
};
use lapin::Connection;
use crate::utils::{hasher, otp};
use uuid::Uuid;
use chrono::Utc;
use redis::AsyncCommands;

/// Plain struct representing the side-effects that need to be persisted
pub struct RegisterEffect {
    pub user_id: Uuid,
    pub full_name: String,
    pub email: String,
    pub phone_number: String,
    pub role_id: i32,
    pub account_status: i32,
    pub hashed_password: String,
    pub is_new: bool,
    pub otp_code: String,
}

/// Pure logic to decide the outcome of a registration attempt.
pub fn decide_register(
    req: UserRegistrationRequest,
    existing_user: Option<&users::Model>,
    pending_status_id: i32,
    customer_role_id: i32,
    hashed_password: String,
) -> Result<RegisterEffect, AppError> {
    // 1. Check existing user
    if let Some(user) = existing_user {
        if user.account_status != pending_status_id { 
            return Err(AppError::Conflict("Email already registered and active".to_string()));
        }
    }

    // 2. Prepare user ID
    let is_new = existing_user.is_none();
    let user_id = if let Some(u) = existing_user {
        u.id
    } else {
        Uuid::new_v4()
    };
    
    // 3. OTP
    let otp_code = otp::generate_6digit_otp();

    Ok(RegisterEffect {
        user_id,
        full_name: req.full_name,
        email: req.email,
        phone_number: req.phone_number,
        role_id: customer_role_id,
        account_status: pending_status_id,
        hashed_password,
        is_new,
        otp_code,
    })
}

pub async fn handle_register(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<Connection>>,
    templates: &HashMap<String, String>,
    req: UserRegistrationRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Fetch data (I/O)
    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing")))?;
    
    let customer_role = roles::Entity::find()
        .filter(roles::Column::Name.eq("Customer"))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role missing")))?;

    let existing = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?;

    // 2. Async logic (Password hashing)
    let hashed_password = hasher::hash_password(req.password.clone()).await?;

    // 3. Decision Logic (Pure)
    let effect = decide_register(req, existing.as_ref(), pending_status.id, customer_role.id, hashed_password)?;

    // 4. Execution (I/O)
    let now = Utc::now();
    let user_active = users::ActiveModel {
        id: Set(effect.user_id),
        full_name: Set(effect.full_name.clone()),
        email: Set(effect.email.clone()),
        password_hash: Set(effect.hashed_password),
        phone_number: Set(effect.phone_number),
        role_id: Set(effect.role_id),
        account_status: Set(effect.account_status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    if effect.is_new {
        user_active.insert(&db).await?;
    } else {
        user_active.update(&db).await?;
    }

    if let Some(mut conn) = valkey {
        let valkey_key = format!("register_verification:{}", effect.email);
        let valkey_data = serde_json::json!({ "code": effect.otp_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(&rmq, templates, &effect.email, &effect.full_name, &effect.otp_code).await?;
    }

    Ok(ApiResponse::message_only(201, "Registration successful"))
}

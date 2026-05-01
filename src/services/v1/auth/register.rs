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
    infrastructure::mq::RabbitMQClient,
};
use crate::utils::{hasher, otp};
use uuid::Uuid;
use chrono::Utc;
use redis::AsyncCommands;

pub async fn handle_register(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<RabbitMQClient>>,
    templates: &HashMap<String, String>,
    req: UserRegistrationRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Load required statuses/roles first
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

    // 2. Check existing user
    let existing = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?;

    if let Some(user) = existing.as_ref() {
        if user.account_status != pending_status.id { 
            return Err(AppError::Conflict("Email already registered and active".to_string()));
        }
    }

    // 3. Hash password
    let hashed_password = hasher::hash_password(req.password).await?;

    // 4. Save user (Update if pending, otherwise Create)
    let user_id = if let Some(u) = existing {
        u.id
    } else {
        Uuid::new_v4()
    };
    
    let now = Utc::now();
    let user_active = users::ActiveModel {
        id: Set(user_id),
        full_name: Set(req.full_name.clone()),
        email: Set(req.email.clone()),
        password_hash: Set(hashed_password),
        phone_number: Set(req.phone_number.clone()),
        role_id: Set(customer_role.id),
        account_status: Set(pending_status.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let existing_user = users::Entity::find_by_id(user_id).one(&db).await?;
    if existing_user.is_none() {
        user_active.insert(&db).await?;
    } else {
        user_active.update(&db).await?;
    }

    // 5. OTP
    let verification_code = otp::generate_6digit_otp();

    if let Some(mut conn) = valkey {
        let valkey_key = format!("register_verification:{}", req.email);
        let valkey_data = serde_json::json!({ "code": verification_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(&rmq, templates, &req.email, &req.full_name, &verification_code).await?;
    }

    Ok(ApiResponse::message_only(201, "Registration successful"))
}

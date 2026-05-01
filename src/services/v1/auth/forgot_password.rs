use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::auth::forgot_password_request::ForgotPasswordRequest,
        responses::base::ApiResponse,
    },
    services::v1::core::email_service,
};
use lapin::Connection;
use crate::utils::otp;
use redis::AsyncCommands;

pub async fn handle_forgot_password(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<Connection>>,
    templates: &HashMap<String, String>,
    req: ForgotPasswordRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let otp_code = otp::generate_6digit_otp();

    if let Some(mut conn) = valkey {
        let valkey_key = format!("forgot_password_verification:{}", req.email);
        let valkey_data = serde_json::json!({ "code": otp_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_forgot_password_email(&rmq, templates, &req.email, &user.full_name, &otp_code).await?;
    }

    Ok(ApiResponse::message_only(200, "OTP sent"))
}

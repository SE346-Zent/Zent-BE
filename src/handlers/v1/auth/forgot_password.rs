use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use validator::Validate;
use sea_orm::{QueryFilter, ColumnTrait};
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::services::v1::auth::forgot_password;
use crate::services::v1::core::email_service;
use crate::model::requests::auth::forgot_password_request::ForgotPasswordRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "OTP sent successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn forgot_password_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    use sea_orm::EntityTrait;
    use crate::entities::users;
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref()).await?;

    let effect = forgot_password::decide_forgot_password(user.as_ref(), payload)?;

    if let Some(effect) = effect {
        use redis::AsyncCommands;
        if let Some(client) = valkey_client {
            let mut conn = client.get_connection();
            let valkey_key = format!("forgot_password_verification:{}", effect.email);
            let valkey_data = serde_json::json!({ "code": effect.otp_code, "attempts": 5 }).to_string();
            conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
        }
        if let Some(rmq) = rabbitmq {
            email_service::send_forgot_password_email(&rmq, &templates, &effect.email, &effect.full_name, &effect.otp_code).await?;
        }
    }

    Ok(Json(ApiResponse::message_only(200, "OTP sent")))
}

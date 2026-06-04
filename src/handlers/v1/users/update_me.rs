use axum::{extract::{State, ConnectInfo}, http::HeaderMap, Json};
use std::net::SocketAddr;
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, Set};
use chrono::Utc;
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::{profile_update_audit_logs, roles::Role},
    extractor::auth_user::AuthUser,
    model::requests::users::ProfileUpdateRequest,
    model::responses::base::ApiResponse,
    model::responses::users::MeResponseData,
    services::v1::users::update_me,
};

#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "users",
    request_body = ProfileUpdateRequest,
    responses(
        (status = 200, description = "Update profile successful", body = ApiResponse<MeResponseData>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn update_me_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<ProfileUpdateRequest>,
) -> Result<Json<ApiResponse<MeResponseData>>, AppError> {
    let user_id = user.id;
    let role_id = user.role_id;
    let effect = update_me::decide_update_me(user, payload)?;

    effect.user_active_model.update(db.as_ref()).await?;

    // Audit log for staff profile updates (Technician=4, Admin=2, SuperAdmin=1)
    if matches!(role_id, r if r == Role::Technician as i32 || r == Role::Admin as i32 || r == Role::SuperAdmin as i32) {
        // Only log if tracked fields actually changed
        if effect.old_values.full_name != effect.new_values.full_name
            || effect.old_values.email != effect.new_values.email
            || effect.old_values.phone_number != effect.new_values.phone_number
        {
            let ip = headers.get("X-Real-IP").and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .unwrap_or_else(|| addr.ip().to_string());

            let audit = profile_update_audit_logs::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(user_id),
                role_id: Set(role_id),
                changed_by: Set(user_id.to_string()),
                old_values: Set(serde_json::to_string(&effect.old_values).unwrap_or_else(|_| "{}".to_string())),
                new_values: Set(serde_json::to_string(&effect.new_values).unwrap_or_else(|_| "{}".to_string())),
                ip_address: Set(Some(ip)),
                created_at: Set(Utc::now()),
            };

            if let Err(e) = audit.insert(db.as_ref()).await {
                tracing::error!(
                    user_id = %user_id,
                    error = %e,
                    "Failed to insert profile update audit log"
                );
            }
        }
    }

    Ok(Json(ApiResponse::success(200, "Update profile successful", effect.response_data)))
}

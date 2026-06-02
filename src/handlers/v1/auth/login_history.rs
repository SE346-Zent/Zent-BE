use std::sync::Arc;

use axum::{extract::State, Json};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::{
    core::errors::{AppError, ErrorResponse},
    entities::login_audit_logs,
    extractor::auth_user::AuthUser,
    model::{responses::{auth::login_history_response::LoginHistoryEntry, base::ApiResponse}},
    services::v1::auth::login_history as login_history_svc,
};

#[utoipa::path(
    get,
    path = "/api/v1/auth/login-history",
    responses(
        (status = 200, description = "Login history", body = ApiResponse<Vec<LoginHistoryEntry>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn login_history_handler(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<LoginHistoryEntry>>>, AppError> {
    let records = login_audit_logs::Entity::find()
        .filter(login_audit_logs::Column::UserId.eq(auth.user.id))
        .order_by_desc(login_audit_logs::Column::CreatedAt)
        .all(db.as_ref())
        .await?;

    let entries = login_history_svc::decide_login_history(records);

    Ok(Json(ApiResponse::success(200, "Login history retrieved successfully", entries)))
}
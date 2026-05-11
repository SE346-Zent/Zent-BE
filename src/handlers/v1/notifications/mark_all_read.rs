use axum::extract::State;
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    post, path = "/api/v1/notifications/read-all",
    responses(
        (status = 200, description = "All marked as read", body = ApiResponse<()>),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn mark_all_read(
    auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // In a real app, update MongoDB.
    let mut mock_records = vec![];
    crate::services::v1::notifications::mark_all_read::mark_all_read(&mut mock_records, auth.user.id);
    Ok(Json(ApiResponse::message_only(200, "All marked as read")))
}

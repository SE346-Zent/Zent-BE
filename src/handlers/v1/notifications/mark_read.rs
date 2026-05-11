use axum::extract::{State, Path};
use axum::Json;
use uuid::Uuid;
use crate::core::errors::AppError;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    post, path = "/api/v1/notifications/{id}/read",
    responses(
        (status = 200, description = "Marked as read", body = MessageOnlyResponse),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn mark_read(
    auth: AuthUser,
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // In a real app, update MongoDB.
    let mut mock_records = vec![];
    crate::services::v1::notifications::mark_read::mark_read(&mut mock_records, auth.user.id, id)?;
    Ok(Json(ApiResponse::message_only(200, "Marked as read")))
}

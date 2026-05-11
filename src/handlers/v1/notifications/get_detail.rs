use axum::extract::{State, Path};
use axum::Json;
use uuid::Uuid;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::notifications::notification_detail_response::NotificationDetailResponse;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    get, path = "/api/v1/notifications/{id}",
    responses(
        (status = 200, description = "Notification details", body = ApiResponse<NotificationDetailResponse>),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_detail(
    auth: AuthUser,
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<NotificationDetailResponse>>, AppError> {
    // In a real app, fetch from MongoDB.
    let mock_records = vec![];
    let data = crate::services::v1::notifications::get_detail::get_detail(&mock_records, auth.user.id, id)?;
    Ok(Json(ApiResponse::success(200, "Notification details retrieved", data)))
}

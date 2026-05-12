use axum::extract::State;
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::notifications::notification_list_response::NotificationListItem;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    post, path = "/api/v1/notifications/outbox/sync",
    responses(
        (status = 200, description = "Synced outbox", body = ApiResponse<Vec<NotificationListItem>>),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "notifications",
    security(("bearer_auth" = []))
)]
pub async fn sync_outbox(
    auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<NotificationListItem>>>, AppError> {
    // In a real app, sync with outbox collection.
    let mut mock_outbox = vec![];
    let mock_records = vec![];
    let data = crate::services::v1::notifications::sync_outbox::sync_outbox(&mut mock_outbox, &mock_records, auth.user.id);
    Ok(Json(ApiResponse::success(200, "Outbox synced", data)))
}

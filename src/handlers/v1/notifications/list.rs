use axum::extract::{State, Query};
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::notifications::notification_list_response::NotificationListItem;
use crate::model::requests::notifications::list_query::NotificationListQuery;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    get, path = "/api/v1/notifications",
    params(NotificationListQuery),
    responses(
        (status = 200, description = "List notifications", body = ApiResponse<Vec<NotificationListItem>>),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list(
    _auth: AuthUser,
    State(_state): State<AppState>,
    Query(query): Query<NotificationListQuery>,
) -> Result<Json<ApiResponse<Vec<NotificationListItem>>>, AppError> {
    // In a real app, you'd fetch from MongoDB.
    let mock_records = vec![];
    let (data, _meta) = crate::services::v1::notifications::list::list_notifications(&mock_records, &query);
    Ok(Json(ApiResponse::success(200, "Notifications retrieved", data)))
}

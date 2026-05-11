use axum::extract::State;
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::notifications::notification_category_response::NotificationCategoryResponse;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    get, path = "/api/v1/notifications/categories",
    responses(
        (status = 200, description = "List notification categories", body = ApiResponse<Vec<NotificationCategoryResponse>>),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_categories(
    _auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<NotificationCategoryResponse>>>, AppError> {
    let data = crate::services::v1::notifications::list_categories::list_categories();
    Ok(Json(ApiResponse::success(200, "Categories retrieved", data)))
}

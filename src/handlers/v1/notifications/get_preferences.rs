use axum::extract::State;
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::notifications::preference_response::NotificationPreferenceResponse;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    get, path = "/api/v1/notifications/preferences",
    responses(
        (status = 200, description = "Notification preferences", body = ApiResponse<Vec<NotificationPreferenceResponse>>),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_preferences(
    _auth: AuthUser,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<NotificationPreferenceResponse>>>, AppError> {
    // In a real app, you'd fetch from DB. For now, using mock logic via service.
    let mock_prefs = std::collections::HashMap::new();
    let data = crate::services::v1::notifications::get_preferences::get_preferences(&mock_prefs);
    Ok(Json(ApiResponse::success(200, "Preferences retrieved", data)))
}

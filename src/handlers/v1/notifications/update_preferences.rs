use axum::extract::State;
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::requests::notifications::update_preference_request::UpdateNotificationPreferenceRequest;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    put, path = "/api/v1/notifications/preferences",
    request_body = UpdateNotificationPreferenceRequest,
    responses(
        (status = 200, description = "Preferences updated", body = ApiResponse<()>),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_preferences(
    _auth: AuthUser,
    State(_state): State<AppState>,
    Json(payload): Json<UpdateNotificationPreferenceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // In a real app, you'd update the DB.
    let mut mock_prefs = std::collections::HashMap::new();
    crate::services::v1::notifications::update_preference::update_preference(payload.category_id, payload.os_enabled, &mut mock_prefs)?;
    Ok(Json(ApiResponse::message_only(200, "Preferences updated")))
}

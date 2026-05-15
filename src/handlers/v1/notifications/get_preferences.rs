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
    tag = "notifications",
    security(("bearer_auth" = []))
)]
pub async fn get_preferences(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<NotificationPreferenceResponse>>>, AppError> {
    let collection = state.mongodb.collection::<mongodb::bson::Document>("user_preferences");
    let doc = collection
        .find_one(mongodb::bson::doc! { "_id": auth.user.id.to_string() })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

    let mut mock_prefs = std::collections::HashMap::new();
    if let Some(d) = doc {
        if let Ok(prefs_array) = d.get_array("preferences") {
            for item in prefs_array {
                if let Some(obj) = item.as_document() {
                    if let (Ok(cat_id), Ok(os_enabled)) = (obj.get_i32("category_id"), obj.get_bool("os_enabled")) {
                        mock_prefs.insert(cat_id, os_enabled);
                    }
                }
            }
        }
    }

    let role_name_lc = auth.role.name.to_lowercase();
    let allowed_ids = state.lookup_tables.notification_categories_by_role.get(&role_name_lc)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let data = crate::services::v1::notifications::get_preferences::get_preferences(&mock_prefs, allowed_ids);
    Ok(Json(ApiResponse::success(200, "Preferences retrieved", data)))
}

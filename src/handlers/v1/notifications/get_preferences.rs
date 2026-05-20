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
/// Handle requests to retrieve the current user's notification delivery preferences.
///
/// This handler fetches the user's preference document from MongoDB, parses the
/// stored overrides, and maps them against the categories permitted for the user's role.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user.
/// * `app_state` - Shared application state containing the MongoDB database and lookup tables.
///
/// # Returns
/// A result containing the successful `ApiResponse` with the list of preferences, or an `AppError`.
pub async fn get_preferences(
    authenticated_user: AuthUser,
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<NotificationPreferenceResponse>>>, AppError> {
    let preferences_collection = app_state.mongodb.collection::<mongodb::bson::Document>("user_preferences");
    let preferences_document = preferences_collection
        .find_one(mongodb::bson::doc! { "_id": authenticated_user.user.id.to_string() })
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("Database error: {}", err)))?;

    let mut user_preference_map = std::collections::HashMap::new();
    if let Some(preference_doc) = preferences_document {
        if let Ok(preferences_array) = preference_doc.get_array("preferences") {
            for item in preferences_array {
                if let Some(preference_obj) = item.as_document() {
                    if let (Ok(category_id), Ok(is_os_enabled)) = (preference_obj.get_i32("category_id"), preference_obj.get_bool("os_enabled")) {
                        user_preference_map.insert(category_id, is_os_enabled);
                    }
                }
            }
        }
    }

    let user_role_name = authenticated_user.role.name.to_lowercase();
    let permitted_category_ids = app_state.lookup_tables.notification_categories_by_role.get(&user_role_name)
        .map(|category_ids| category_ids.as_slice())
        .unwrap_or(&[]);

    let preferences_data = crate::services::v1::notifications::get_preferences::get_preferences(&user_preference_map, permitted_category_ids);
    Ok(Json(ApiResponse::success(200, "Preferences retrieved", preferences_data)))
}

use axum::extract::State;
use axum::Json;
use crate::core::errors::AppError;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};
use crate::model::requests::notifications::update_preference_request::UpdateNotificationPreferenceRequest;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

#[utoipa::path(
    put, path = "/api/v1/notifications/preferences",
    request_body = UpdateNotificationPreferenceRequest,
    responses(
        (status = 200, description = "Preferences updated", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "notifications",
    security(("bearer_auth" = []))
)]
/// Handle requests to update a user's notification delivery preferences.
///
/// This handler retrieves the user's existing preferences from MongoDB,
/// validates the requested change against role-based category permissions,
/// and performs an upsert to save the updated preference set.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user.
/// * `app_state` - Shared application state containing the MongoDB database and lookup tables.
/// * `update_payload` - The request containing the category ID and the new OS-delivery status.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn update_preferences(
    authenticated_user: AuthUser,
    State(app_state): State<AppState>,
    Json(update_payload): Json<UpdateNotificationPreferenceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let preferences_collection = app_state.mongodb.collection::<mongodb::bson::Document>("user_preferences");
    let target_user_id_string = authenticated_user.user.id.to_string();

    let existing_preferences_document = preferences_collection
        .find_one(mongodb::bson::doc! { "_id": &target_user_id_string })
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("Database error: {}", err)))?;

    let mut user_preference_map = std::collections::HashMap::new();
    if let Some(preference_doc) = existing_preferences_document {
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

    crate::services::v1::notifications::update_preference::update_preference(
        update_payload.category_id,
        update_payload.os_enabled,
        &mut user_preference_map,
        permitted_category_ids,
    )?;

    let serialized_preferences: Vec<mongodb::bson::Bson> = user_preference_map
        .into_iter()
        .map(|(category_id, is_os_enabled)| {
            mongodb::bson::Bson::Document(mongodb::bson::doc! {
                "category_id": category_id,
                "os_enabled": is_os_enabled
            })
        })
        .collect();

    let updated_document = mongodb::bson::doc! {
        "_id": &target_user_id_string,
        "preferences": serialized_preferences
    };

    let upsert_options = mongodb::options::ReplaceOptions::builder().upsert(true).build();
    preferences_collection
        .replace_one(mongodb::bson::doc! { "_id": &target_user_id_string }, updated_document)
        .with_options(upsert_options)
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("Failed to save preferences: {}", err)))?;

    Ok(Json(ApiResponse::message_only(200, "Preferences updated")))
}

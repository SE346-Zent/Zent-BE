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
    security(("bearer_auth" = []))
)]
pub async fn update_preferences(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateNotificationPreferenceRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let collection = state.mongodb.collection::<mongodb::bson::Document>("user_preferences");
    let user_id_str = auth.user.id.to_string();

    let doc = collection
        .find_one(mongodb::bson::doc! { "_id": &user_id_str })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

    let mut user_prefs = std::collections::HashMap::new();
    if let Some(d) = doc {
        if let Ok(prefs_array) = d.get_array("preferences") {
            for item in prefs_array {
                if let Some(obj) = item.as_document() {
                    if let (Ok(cat_id), Ok(os_enabled)) = (obj.get_i32("category_id"), obj.get_bool("os_enabled")) {
                        user_prefs.insert(cat_id, os_enabled);
                    }
                }
            }
        }
    }

    crate::services::v1::notifications::update_preference::update_preference(
        payload.category_id,
        payload.os_enabled,
        &mut user_prefs,
    )?;

    let updated_array: Vec<mongodb::bson::Bson> = user_prefs
        .into_iter()
        .map(|(cat_id, os_enabled)| {
            mongodb::bson::Bson::Document(mongodb::bson::doc! {
                "category_id": cat_id,
                "os_enabled": os_enabled
            })
        })
        .collect();

    let new_doc = mongodb::bson::doc! {
        "_id": &user_id_str,
        "preferences": updated_array
    };

    let options = mongodb::options::ReplaceOptions::builder().upsert(true).build();
    collection
        .replace_one(mongodb::bson::doc! { "_id": &user_id_str }, new_doc)
        .with_options(options)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save preferences: {}", e)))?;

    Ok(Json(ApiResponse::message_only(200, "Preferences updated")))
}

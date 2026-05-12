use axum::extract::{State, Query};
use axum::Json;
use futures::TryStreamExt;
use mongodb::bson::doc;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::notifications::notification_list_response::NotificationListItem;
use crate::model::requests::notifications::list_query::NotificationListQuery;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;
use crate::services::v1::notifications::NotificationRecord;

#[utoipa::path(
    get, path = "/api/v1/notifications",
    params(NotificationListQuery),
    responses(
        (status = 200, description = "List notifications", body = ApiResponse<Vec<NotificationListItem>>),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "notifications",
    security(("bearer_auth" = []))
)]
pub async fn list(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<NotificationListQuery>,
) -> Result<Json<ApiResponse<Vec<NotificationListItem>>>, AppError> {
    // 1. Fetch all notifications for this user from MongoDB
    let collection = state.mongodb
        .collection::<mongodb::bson::Document>("notifications");

    let filter = doc! { "user_id": auth.user.id.to_string() };
    let options = mongodb::options::FindOptions::builder()
        .sort(doc! { "created_at": -1 })
        .build();

    let cursor = collection
        .find(filter)
        .with_options(options)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let docs: Vec<mongodb::bson::Document> = cursor
        .try_collect()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Convert BSON documents to NotificationRecords
    let records: Vec<NotificationRecord> = docs.iter()
        .filter_map(|doc| {
            let notification_id = doc.get_str("notification_id").ok()?.parse().ok()?;
            let user_id = doc.get_str("user_id").ok()?.parse().ok()?;
            let category_id = doc.get_i32("category_id").ok()?;
            let title = doc.get_str("title").ok()?.to_string();
            let body = doc.get_str("body").ok()?.to_string();
            let data = doc.get("data")
                .and_then(|d| {
                    let bson_val: mongodb::bson::Bson = d.clone();
                    let json_val: serde_json::Value = mongodb::bson::from_bson(bson_val).ok()?;
                    Some(json_val)
                })
                .unwrap_or(serde_json::Value::Null);
            let is_read = doc.get_bool("is_read").unwrap_or(false);
            let os_notification_id = doc.get_str("os_notification_id")
                .ok()
                .and_then(|s| s.parse::<uuid::Uuid>().ok());
            let created_at = doc.get_datetime("created_at")
                .ok()
                .map(|dt| {
                    let millis = dt.timestamp_millis();
                    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)
                        .unwrap_or_default()
                })?;

            Some(NotificationRecord {
                notification_id,
                user_id,
                category_id,
                title,
                body,
                data,
                is_read,
                os_notification_id,
                created_at,
            })
        })
        .collect();

    // 2. If customer, fetch disabled category ids from preferences
    let is_customer = auth.role.name.to_lowercase() == "customer";
    let disabled_category_ids: Vec<i32> = if is_customer {
        let prefs_collection = state.mongodb
            .collection::<mongodb::bson::Document>("user_preferences");
        let pref_doc = prefs_collection
            .find_one(doc! { "_id": auth.user.id.to_string() })
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

        let mut disabled = Vec::new();
        if let Some(d) = pref_doc {
            if let Ok(prefs_array) = d.get_array("preferences") {
                for item in prefs_array {
                    if let Some(obj) = item.as_document() {
                        if let (Ok(cat_id), Ok(os_enabled)) =
                            (obj.get_i32("category_id"), obj.get_bool("os_enabled"))
                        {
                            if !os_enabled {
                                disabled.push(cat_id);
                            }
                        }
                    }
                }
            }
        }
        disabled
    } else {
        vec![]
    };

    // 3. Pass to pure service logic
    let (data, meta) = crate::services::v1::notifications::list::list_notifications(
        &records,
        &query,
        &disabled_category_ids,
    );

    Ok(Json(ApiResponse::success_with_meta(200, "Notifications retrieved", data, meta)))
}

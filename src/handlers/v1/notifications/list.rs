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
/// Handle requests to retrieve a paginated list of notifications for the authenticated user.
///
/// This handler retrieves notification buckets from MongoDB, flattens them,
/// applies pagination and filtering via the service layer, and as a side-effect,
/// marks all returned notifications as 'read' while decrementing the unread
/// cache in Valkey.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user.
/// * `app_state` - Shared application state containing MongoDB and Valkey.
/// * `list_query` - Query parameters for filtering by category and pagination.
///
/// # Returns
/// A result containing the successful `ApiResponse` with notification items and metadata, or an `AppError`.
pub async fn list(
    authenticated_user: AuthUser,
    State(app_state): State<AppState>,
    Query(list_query): Query<NotificationListQuery>,
) -> Result<Json<ApiResponse<Vec<NotificationListItem>>>, AppError> {

    // page and limit are Option<u32> — reject explicit zero, allow None (defaults apply)
    if list_query.page == Some(0) {
        return Err(AppError::BadRequest("Page must be greater than 0".to_string()));
    }

    if list_query.limit == Some(0) {
        return Err(AppError::BadRequest("Limit must be greater than 0".to_string()));
    }

    // 1. Fetch all notifications for this user from MongoDB
    let notification_collection = app_state.mongodb
        .collection::<mongodb::bson::Document>("notifications");

    // Bucket-pattern: each document holds a `notifications` array.
    // Flatten all buckets for this user into a single list.
    let search_filter = doc! { "user_id": authenticated_user.user.id.to_string() };

    let cursor = notification_collection
        .find(search_filter)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let bucket_docs: Vec<mongodb::bson::Document> = cursor
        .try_collect()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Flatten bucket documents to NotificationRecords
    let notification_records: Vec<NotificationRecord> = bucket_docs
        .iter()
        .flat_map(|bucket_doc| {
            let bucket_user_id = bucket_doc
                .get_str("user_id")
                .ok()
                .and_then(|s| s.parse::<uuid::Uuid>().ok());

            let notifications_array = bucket_doc.get_array("notifications").ok();

            match (bucket_user_id, notifications_array) {
                (Some(user_id), Some(arr)) => {
                    // Collect into Vec to satisfy the iterator bound
                    arr.iter()
                        .filter_map(|item| {
                            let notif_doc = item.as_document()?;
                            let notification_id =
                                notif_doc.get_str("notification_id").ok()?.parse().ok()?;
                            let category_id = notif_doc.get_i32("category_id").ok()?;
                            let title = notif_doc.get_str("title").ok()?.to_string();
                            let body = notif_doc.get_str("body").ok()?.to_string();
                            let data = notif_doc
                                .get("data")
                                .and_then(|d| {
                                    let bson_val: mongodb::bson::Bson = d.clone();
                                    mongodb::bson::from_bson(bson_val).ok()
                                })
                                .unwrap_or(serde_json::Value::Null);
                            let is_read = notif_doc.get_bool("is_read").unwrap_or(false);
                            let os_notification_id = notif_doc
                                .get_str("os_notification_id")
                                .ok()
                                .and_then(|s| s.parse::<uuid::Uuid>().ok());
                            let created_at = notif_doc
                                .get_datetime("created_at")
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
                        .collect::<Vec<_>>()
                        .into_iter()
                }
                _ => Vec::new().into_iter(),
            }
        })
        .collect();

    // 2. Pass to pure service logic (no preference filtering — all notifications shown)
    let (notification_items, pagination_meta) = crate::services::v1::notifications::list::list_notifications(
        &notification_records,
        &list_query,
    );

    // 3. Mark returned notifications as read in MongoDB and decrement Valkey cache
    let unread_notification_ids: Vec<String> = notification_items
        .iter()
        .filter(|item| !item.is_read)
        .map(|item| item.notification_id.clone())
        .collect();

    let unread_count = unread_notification_ids.len();

    if !unread_notification_ids.is_empty() {
        // Mark as read in MongoDB using arrayFilters
        let _ = notification_collection
            .update_many(
                doc! {
                    "user_id": authenticated_user.user.id.to_string(),
                    "notifications.notification_id": { "$in": &unread_notification_ids },
                },
                doc! {
                    "$set": { "notifications.$[elem].is_read": true },
                },
            )
            .array_filters(vec![
                doc! { "elem.notification_id": { "$in": &unread_notification_ids } },
            ])
            .await;

        // Decrement Valkey unread counter
        if let Some(valkey_instance) = &app_state.valkey {
            if let Ok(mut valkey_conn) = valkey_instance.get_connection().await {
                let _ = redis::cmd("DECRBY")
                    .arg(format!("unread:{}", authenticated_user.user.id))
                    .arg(unread_count as i64)
                    .query_async::<()>(&mut valkey_conn)
                    .await;
            }
        }
    }

    Ok(Json(ApiResponse::success_with_meta(200, "Notifications retrieved", notification_items, pagination_meta)))
}

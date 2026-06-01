use axum::extract::State;
use axum::Json;
use futures::TryStreamExt;
use mongodb::bson::doc;
use crate::core::errors::AppError;
use crate::model::responses::base::ApiResponse;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;

/// GET /api/v1/notifications/unread-count
///
/// Returns the number of unread notifications for the authenticated user.
///
/// Queries Valkey (key: `unread:{user_id}`) first. On cache miss,
/// falls back to MongoDB by counting notifications where `is_read = false`,
/// then writes the count to Valkey for future requests.
#[utoipa::path(
    get, path = "/api/v1/notifications/unread-count",
    responses(
        (status = 200, description = "Unread notification count", body = ApiResponse<u64>),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "notifications",
    security(("bearer_auth" = []))
)]
/// Handle requests to retrieve the total count of unread notifications for the authenticated user.
///
/// This handler first checks the Valkey cache (key: `unread:{user_id}`). If the
/// count is not cached, it falls back to MongoDB, iterates through notification
/// buckets for the user, and sums the unread entries before updating the cache.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user.
/// * `app_state` - Shared application state containing Valkey and MongoDB connections.
///
/// # Returns
/// A result containing the successful `ApiResponse` with the unread count, or an `AppError`.
pub async fn get_unread_noti_count(
    authenticated_user: AuthUser,
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<u64>>, AppError> {
    let target_user_id = authenticated_user.user.id;
    let cache_key_name = format!("unread:{}", target_user_id);

    // 1. Try Valkey first
    if let Some(valkey_instance) = &app_state.valkey {
        if let Ok(mut valkey_conn) = valkey_instance.get_connection().await {
            match redis::cmd("GET")
                .arg(&cache_key_name)
                .query_async::<Option<i64>>(&mut valkey_conn)
                .await
            {
                Ok(Some(cached_count)) if cached_count >= 0 => {
                    return Ok(Json(ApiResponse::success(
                        200,
                        "Unread count retrieved",
                        cached_count as u64,
                    )));
                }
                Ok(_) => {
                    // Key missing or malformed → fallback to MongoDB
                }
                Err(err) => {
                    tracing::warn!("Valkey GET failed for {}: {:?}. Falling back to MongoDB.", cache_key_name, err);
                }
            }
        }
    }

    // 2. Fallback: count unread notifications from MongoDB
    let notification_collection = app_state.mongodb
        .collection::<mongodb::bson::Document>("notifications");

    // Bucket pattern: iterate all buckets for this user, count unread notifications
    let search_filter = doc! { "user_id": target_user_id.to_string() };
    let cursor = notification_collection
        .find(search_filter)
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("MongoDB error: {}", err)))?;

    let bucket_docs: Vec<mongodb::bson::Document> = cursor
        .try_collect()
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("MongoDB error: {}", err)))?;

    let mut unread_total_count: u64 = 0;
    for bucket_document in &bucket_docs {
        let notification_array: Vec<&mongodb::bson::Bson> = bucket_document
            .get_array("notifications")
            .map(|arr| arr.iter().collect())
            .unwrap_or_default();

        for item in notification_array {
            if let Some(notification_doc) = item.as_document() {
                if !notification_doc.get_bool("is_read").unwrap_or(false) {
                    unread_total_count += 1;
                }
            }
        }
    }

    // 3. Write count to Valkey for future requests
    if let Some(valkey_instance) = &app_state.valkey {
        if let Ok(mut valkey_conn) = valkey_instance.get_connection().await {
            let _ = redis::cmd("SET")
                .arg(&cache_key_name)
                .arg(unread_total_count as i64)
                .query_async::<()>(&mut valkey_conn)
                .await;
        }
    }

    Ok(Json(ApiResponse::success(
        200,
        "Unread count retrieved",
        unread_total_count,
    )))
}

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
pub async fn get_unread_noti_count(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<u64>>, AppError> {
    let user_id = auth.user.id;
    let valkey_key = format!("unread:{}", user_id);

    // 1. Try Valkey first
    if let Some(valkey) = &state.valkey {
        let mut conn = valkey.get_connection();
        match redis::cmd("GET")
            .arg(&valkey_key)
            .query_async::<Option<i64>>(&mut conn)
            .await
        {
            Ok(Some(count)) if count >= 0 => {
                return Ok(Json(ApiResponse::success(
                    200,
                    "Unread count retrieved",
                    count as u64,
                )));
            }
            Ok(_) => {
                // Key doesn't exist or has unexpected value → fallback to MongoDB
            }
            Err(e) => {
                tracing::warn!("Valkey GET failed for {}: {:?}. Falling back to MongoDB.", valkey_key, e);
            }
        }
    }

    // 2. Fallback: count unread notifications from MongoDB
    let collection = state.mongodb
        .collection::<mongodb::bson::Document>("notifications");

    // Bucket pattern: iterate all buckets for this user, count unread notifications
    let filter = doc! { "user_id": user_id.to_string() };
    let cursor = collection
        .find(filter)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("MongoDB error: {}", e)))?;

    let bucket_docs: Vec<mongodb::bson::Document> = cursor
        .try_collect()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("MongoDB error: {}", e)))?;

    let mut unread_count: u64 = 0;
    for bucket_doc in &bucket_docs {
        let notifs: Vec<&mongodb::bson::Bson> = bucket_doc
            .get_array("notifications")
            .map(|arr| arr.iter().collect())
            .unwrap_or_default();

        for item in notifs {
            if let Some(notif_doc) = item.as_document() {
                if !notif_doc.get_bool("is_read").unwrap_or(false) {
                    unread_count += 1;
                }
            }
        }
    }

    // 3. Write count to Valkey for future requests
    if let Some(valkey) = &state.valkey {
        let mut conn = valkey.get_connection();
        let _ = redis::cmd("SET")
            .arg(&valkey_key)
            .arg(unread_count as i64)
            .query_async::<()>(&mut conn)
            .await;
    }

    Ok(Json(ApiResponse::success(
        200,
        "Unread count retrieved",
        unread_count,
    )))
}

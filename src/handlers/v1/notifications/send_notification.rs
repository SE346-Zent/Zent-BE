use std::sync::Arc;
use chrono::Utc;
use mongodb::bson::{doc, DateTime as BsonDateTime, to_bson};
use mongodb::options::FindOptions;
use futures::TryStreamExt;
use sea_orm::{ActiveModelTrait, Set, DatabaseConnection};
use uuid::Uuid;
use tracing::{info, warn};
use crate::core::errors::AppError;
use crate::entities::outbox_records;
use crate::infrastructure::cache::ValkeyClient;

/// Dispatch a notification to a specific user.
///
/// **chat_message category (FCM-only, no bell icon):**
/// Chat messages are never saved to MongoDB (they don't appear in the
/// bell-icon notification list). An FCM push is sent only when the user
/// is offline (not connected via WebSocket). When online, the message
/// is delivered in real-time via WebSocket — no push needed.
///
/// **All other categories (bell icon + optional FCM):**
/// Always saved to MongoDB (in-app bell-icon notification list) and
/// the Valkey unread counter is incremented. Then checks the user's
/// notification preferences: if push is enabled for this category
/// (or no preferences set — default enabled), creates an outbox record
/// which triggers FCM push delivery via the relay → consumer pipeline.
/// No online/offline gating — FCM is always sent when push is enabled.
///
/// `category_slug` must be one of the slugs in `NOTIFICATION_CATEGORIES`.
pub async fn send_notification(
    mongodb_db: &mongodb::Database,
    valkey_client: Option<Arc<ValkeyClient>>,
    db_connection: &DatabaseConnection,
    recipient_user_id: Uuid,
    category_slug: &str,
    notification_title: &str,
    notification_body: &str,
    notification_data: serde_json::Value,
) -> Result<(), AppError> {
    // ── Resolve category id ────────────────────────────────────────
    let category_id = crate::services::v1::notifications::find_category_id_by_slug(category_slug)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid category slug: {}", category_slug)))?;

    let notification_id = Uuid::new_v4();
    let current_timestamp = Utc::now();

    // ── chat_message: FCM-only, no bell icon ───────────────────────
    if category_slug == "chat_message" {
        // Never save chat messages to MongoDB (no bell icon listing)
        // Only send FCM push when the user is offline
        let ws_manager = crate::infrastructure::ws::get_ws_manager();
        if ws_manager.is_connected(&user_id).await {
            info!("User {} is online — chat message delivered via WS, skipping FCM", user_id);
            return Ok(());
        }
        // User is offline: check preferences, then send FCM
        if !is_push_enabled_for_user(mongodb, user_id, category_id).await {
            info!("Push disabled for user {} — skipping chat FCM", user_id);
            return Ok(());
        }
        // Fall through to outbox creation below (no MongoDB save, no Valkey increment)
    } else {
        // ── All other categories: bell icon + optional FCM ─────────
        // 1. Save to MongoDB (in-app bell-icon notification list)
        save_notification_to_mongodb(mongodb, notification_id, user_id, category_id, title, body, &data, now)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save notification to MongoDB: {}", e)))?;

    // ── 2. Increment Valkey unread counter ─────────────────────────
    if let Some(vk) = valkey_client.as_ref() {
        increment_unread_count(vk, recipient_user_id).await;
    }

    // ── 3. Check user preferences — skip outbox if push disabled ──
    if !is_push_enabled_for_user(mongodb_db, recipient_user_id, category_id).await {
        info!("Push disabled for user {} category {} — notification saved to MongoDB only", recipient_user_id, category_id);
        return Ok(());
    }

    // ── 4. Create outbox entry (triggers FCM push) ─────────────────
    let serialized_data = serde_json::to_string(&notification_data)
        .unwrap_or_else(|_| "{}".to_string());

    let outbox_entry = outbox_records::ActiveModel {
        outbox_id: Set(Uuid::new_v4()),
        user_id: Set(recipient_user_id),
        notification_id: Set(notification_id),
        category_id: Set(category_id),
        title: Set(notification_title.to_string()),
        body: Set(notification_body.to_string()),
        data: Set(serialized_data),
        created_at: Set(current_timestamp),
        delivered: Set(false),
    };

    outbox_entry
        .insert(db_connection)
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("Failed to insert outbox record: {}", err)))?;

    Ok(())
}

/// Save a notification document into MongoDB using the bucket pattern.
async fn save_notification_to_mongodb(
    mongodb_db: &mongodb::Database,
    notification_id: Uuid,
    recipient_user_id: Uuid,
    category_id: i32,
    notification_title: &str,
    notification_body: &str,
    notification_data: &serde_json::Value,
    created_at_timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<(), anyhow::Error> {
    let notification_collection = mongodb_db.collection::<mongodb::bson::Document>("notifications");
    let target_user_id_string = recipient_user_id.to_string();

    let bson_metadata = to_bson(notification_data).unwrap_or_default();

    let notification_item_doc = doc! {
        "notification_id": notification_id.to_string(),
        "category_id": category_id,
        "title": notification_title,
        "body": notification_body,
        "data": bson_metadata,
        "is_read": false,
        "os_notification_id": mongodb::bson::Bson::Null,
        "created_at": BsonDateTime::from_millis(created_at_timestamp.timestamp_millis()),
    };

    // Find the latest bucket for this user
    let find_opts = FindOptions::builder()
        .sort(doc! { "page": -1 })
        .limit(1)
        .build();

    let mut cursor = notification_collection
        .find(doc! { "user_id": &target_user_id_string })
        .with_options(find_opts)
        .await?;

    let latest_bucket: Option<mongodb::bson::Document> = cursor.try_next().await?;

    if let Some(bucket) = latest_bucket {
        let current_count = bucket.get_i32("count").unwrap_or(0);
        let current_page = bucket.get_i32("page").unwrap_or(1);

        if current_count < 30 {
            // Push into existing bucket
            notification_collection
                .update_one(
                    doc! { "user_id": &target_user_id_string, "page": current_page },
                    doc! {
                        "$push": { "notifications": &notification_item_doc },
                        "$inc": { "count": 1 },
                    },
                )
                .await?;
        } else {
            // Bucket full — spill to a new page
            let new_page = current_page + 1;
            let bucket_doc = doc! {
                "user_id": &target_user_id_string,
                "page": new_page,
                "count": 1_i32,
                "notifications": [&notification_item_doc],
            };
            notification_collection.insert_one(bucket_doc).await?;
        }
    } else {
        // No buckets exist yet — create page 1
        let bucket_doc = doc! {
            "user_id": &target_user_id_string,
            "page": 1_i32,
            "count": 1_i32,
            "notifications": [&notification_item_doc],
        };
        notification_collection.insert_one(bucket_doc).await?;
    }

    Ok(())
}

/// Increment the Valkey unread counter for a user.
async fn increment_unread_count(valkey: &ValkeyClient, user_id: Uuid) {
    let key = format!("unread:{}", user_id);
    if let Ok(mut conn) = valkey.get_connection().await {
        if let Err(e) = redis::cmd("INCR")
            .arg(&key)
            .query_async::<i64>(&mut conn)
            .await
        {
            warn!("Failed to increment Valkey unread count for {}: {:?}", user_id, e);
        }
    } else {
        warn!("Valkey unavailable — unread count increment skipped for {}", user_id);
    }
}

/// Check whether the user has push notifications enabled for the given category.
///
/// Looks up the user's preferences from the MongoDB `user_preferences` collection.
/// If no preferences document exists, push is enabled by default.
/// Returns `true` if push is enabled, `false` if disabled.
async fn is_push_enabled_for_user(
    mongodb: &mongodb::Database,
    user_id: Uuid,
    category_id: i32,
) -> bool {
    let collection = mongodb.collection::<mongodb::bson::Document>("user_preferences");

    let doc = match collection
        .find_one(doc! { "_id": user_id.to_string() })
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => return true,  // No preferences → default enabled
        Err(e) => {
            warn!("Failed to read preferences for user {}: {:?}", user_id, e);
            return true;  // Error → default to enabled (fail open)
        }
    };

    let prefs_array = match doc.get_array("preferences") {
        Ok(arr) => arr,
        Err(_) => return true,  // No preferences array → default enabled
    };

    for item in prefs_array {
        if let Some(obj) = item.as_document() {
            if let (Ok(cat_id), Ok(os_enabled)) = (obj.get_i32("category_id"), obj.get_bool("os_enabled")) {
                if cat_id == category_id {
                    return os_enabled;
                }
            }
        }
    }

    true  // No matching preference entry → default enabled
}

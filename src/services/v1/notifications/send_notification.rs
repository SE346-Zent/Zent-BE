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
/// This function executes the multi-stage delivery process:
/// 1. Persists the notification to MongoDB (in-app history) using the bucket pattern.
/// 2. Increments the unread count in Valkey.
/// 3. Checks user preferences to determine if push notification (OS delivery) is enabled.
/// 4. If enabled, creates an outbox record in MySQL for the relay worker to process.
///
/// # Arguments
/// * `mongodb_db` - Shared MongoDB database connection.
/// * `valkey_client` - Optional shared Valkey cache client.
/// * `db_connection` - Shared SQL database connection pool.
/// * `recipient_user_id` - Unique ID of the user receiving the notification.
/// * `category_slug` - Human-readable slug for the notification category.
/// * `notification_title` - The headline of the notification.
/// * `notification_body` - The primary content of the notification.
/// * `notification_data` - Additional JSON metadata associated with the event.
///
/// # Returns
/// A result indicating success (`Ok(())`) or an `AppError`.
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
    let category_id = super::find_category_id_by_slug(category_slug)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid category slug: {}", category_slug)))?;

    let notification_id = Uuid::new_v4();
    let current_timestamp = Utc::now();

    // ── 1. Save to MongoDB (in-app notification list — always) ────
    save_notification_to_mongodb(mongodb_db, notification_id, recipient_user_id, category_id, notification_title, notification_body, &notification_data, current_timestamp)
        .await
        .map_err(|err| AppError::Internal(anyhow::anyhow!("Failed to save notification to MongoDB: {}", err)))?;

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

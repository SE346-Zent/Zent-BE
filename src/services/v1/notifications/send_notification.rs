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

/// Send a notification to a user.
///
/// Always saves to MongoDB (in-app notification list). Then checks the user's
/// notification preferences. If the user has enabled push for this category
/// (or has no preferences set — default enabled), creates an outbox record
/// which triggers FCM push delivery via the relay → consumer pipeline.
/// If push is disabled for this category, no outbox is created and no FCM
/// is sent.
///
/// `category_slug` must be one of the slugs in `NOTIFICATION_CATEGORIES`.
pub async fn send_notification(
    mongodb: &mongodb::Database,
    valkey: Option<Arc<ValkeyClient>>,
    db: &DatabaseConnection,
    user_id: Uuid,
    category_slug: &str,
    title: &str,
    body: &str,
    data: serde_json::Value,
) -> Result<(), AppError> {
    // ── Resolve category id ────────────────────────────────────────
    let category_id = super::find_category_id_by_slug(category_slug)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid category slug: {}", category_slug)))?;

    let notification_id = Uuid::new_v4();
    let now = Utc::now();

    // ── 1. Save to MongoDB (in-app notification list — always) ────
    save_notification_to_mongodb(mongodb, notification_id, user_id, category_id, title, body, &data, now)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save notification to MongoDB: {}", e)))?;

    // ── 2. Increment Valkey unread counter ─────────────────────────
    if let Some(vk) = valkey.as_ref() {
        increment_unread_count(vk, user_id).await;
    }

    // ── 3. Check user preferences — skip outbox if push disabled ──
    if !is_push_enabled_for_user(mongodb, user_id, category_id).await {
        info!("Push disabled for user {} category {} — notification saved to MongoDB only", user_id, category_id);
        return Ok(());
    }

    // ── 4. Create outbox entry (triggers FCM push) ─────────────────
    let data_json = serde_json::to_string(&data)
        .unwrap_or_else(|_| "{}".to_string());

    let outbox_entry = outbox_records::ActiveModel {
        outbox_id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        notification_id: Set(notification_id),
        category_id: Set(category_id),
        title: Set(title.to_string()),
        body: Set(body.to_string()),
        data: Set(data_json),
        created_at: Set(now.into()),
        delivered: Set(false),
    };

    outbox_entry
        .insert(db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to insert outbox record: {}", e)))?;

    Ok(())
}

/// Save a notification document into MongoDB using the bucket pattern.
async fn save_notification_to_mongodb(
    mongodb: &mongodb::Database,
    notification_id: Uuid,
    user_id: Uuid,
    category_id: i32,
    title: &str,
    body: &str,
    data: &serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), anyhow::Error> {
    let notif_collection = mongodb.collection::<mongodb::bson::Document>("notifications");
    let user_id_str = user_id.to_string();

    let bson_data = to_bson(data).unwrap_or_default();

    let notif_item = doc! {
        "notification_id": notification_id.to_string(),
        "category_id": category_id as i32,
        "title": title,
        "body": body,
        "data": bson_data,
        "is_read": false,
        "os_notification_id": mongodb::bson::Bson::Null,
        "created_at": BsonDateTime::from_millis(created_at.timestamp_millis()),
    };

    // Find the latest bucket for this user
    let find_opts = FindOptions::builder()
        .sort(doc! { "page": -1 })
        .limit(1)
        .build();

    let mut cursor = notif_collection
        .find(doc! { "user_id": &user_id_str })
        .with_options(find_opts)
        .await?;

    let latest_bucket: Option<mongodb::bson::Document> = cursor.try_next().await?;

    if let Some(bucket) = latest_bucket {
        let current_count = bucket.get_i32("count").unwrap_or(0);
        let current_page = bucket.get_i32("page").unwrap_or(1);

        if current_count < 30 {
            // Push into existing bucket
            notif_collection
                .update_one(
                    doc! { "user_id": &user_id_str, "page": current_page },
                    doc! {
                        "$push": { "notifications": &notif_item },
                        "$inc": { "count": 1 },
                    },
                )
                .await?;
        } else {
            // Bucket full — spill to a new page
            let new_page = current_page + 1;
            let bucket_doc = doc! {
                "user_id": &user_id_str,
                "page": new_page,
                "count": 1_i32,
                "notifications": [&notif_item],
            };
            notif_collection.insert_one(bucket_doc).await?;
        }
    } else {
        // No buckets exist yet — create page 1
        let bucket_doc = doc! {
            "user_id": &user_id_str,
            "page": 1_i32,
            "count": 1_i32,
            "notifications": [&notif_item],
        };
        notif_collection.insert_one(bucket_doc).await?;
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

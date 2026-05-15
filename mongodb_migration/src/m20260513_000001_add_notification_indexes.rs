use mongodb::{Database, bson::doc, error::Error as MongoError, IndexModel};
use async_trait::async_trait;

use crate::MongoMigration;

/// Additional migration: add query-friendly indexes on the `notifications` collection.
pub struct AddNotificationIndexes;

#[async_trait]
impl MongoMigration for AddNotificationIndexes {
    fn name(&self) -> &str {
        "m20260513_000001_add_notification_indexes"
    }

    async fn up(&self, db: &Database) -> Result<(), MongoError> {
        let notif_col = db.collection::<mongodb::bson::Document>("notifications");

        // Index 1: user_id + created_at (desc) — speeds up list query
        // The list handler does: find({ user_id: "..." }) with sort({ created_at: -1 })
        let idx_user_created = IndexModel::builder()
            .keys(doc! { "user_id": 1, "created_at": -1 })
            .build();
        notif_col.create_index(idx_user_created).await?;

        // Index 2: notification_id (legacy flat-document field; kept for compatibility)
        let idx_notification_id = IndexModel::builder()
            .keys(doc! { "notification_id": 1 })
            .build();
        notif_col.create_index(idx_notification_id).await?;

        // Index 3: notifications.notification_id — multikey index for
        // bucket-pattern sync_outbox lookups (queries the nested array field).
        let idx_nested_notification_id = IndexModel::builder()
            .keys(doc! { "notifications.notification_id": 1 })
            .build();
        notif_col.create_index(idx_nested_notification_id).await?;

        Ok(())
    }

    async fn down(&self, db: &Database) -> Result<(), MongoError> {
        let notif_col = db.collection::<mongodb::bson::Document>("notifications");
        notif_col.drop_index("user_id_1_created_at_-1").await?;
        notif_col.drop_index("notification_id_1").await?;
        Ok(())
    }
}

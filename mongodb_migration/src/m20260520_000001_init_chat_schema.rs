use mongodb::{Database, bson::doc, error::Error as MongoError, IndexModel};
use async_trait::async_trait;
use crate::MongoMigration;

pub struct InitChatSchema;

#[async_trait]
impl MongoMigration for InitChatSchema {
    fn name(&self) -> &str {
        "m20260520_000001_init_chat_schema"
    }

    async fn up(&self, db: &Database) -> Result<(), MongoError> {
        // 1. Create `messages` collection
        let messages_schema = doc! {
            "$jsonSchema": {
                "bsonType": "object",
                "required": ["room_id", "sender_id", "content", "created_at"],
                "properties": {
                    "room_id": { "bsonType": "string" },
                    "sender_id": { "bsonType": "string" },
                    "content": { "bsonType": "string" },
                    "image_url": { "bsonType": ["string", "null"] },
                    "reply_to": { "bsonType": ["string", "null"] },
                    "created_at": { "bsonType": "date" },
                    "edited_at": { "bsonType": ["date", "null"] }
                }
            }
        };
        db.create_collection("messages")
            .validator(messages_schema)
            .await?;

        // Index: room_id + created_at (desc) — for paginated message history
        let msg_col = db.collection::<mongodb::bson::Document>("messages");
        let idx_room_created = IndexModel::builder()
            .keys(doc! { "room_id": 1, "created_at": -1 })
            .build();
        msg_col.create_index(idx_room_created).await?;

        // Index: sender_id — for "my messages" queries
        let idx_sender = IndexModel::builder()
            .keys(doc! { "sender_id": 1 })
            .build();
        msg_col.create_index(idx_sender).await?;

        // 2. Create `read_receipts` collection
        let receipts_schema = doc! {
            "$jsonSchema": {
                "bsonType": "object",
                "required": ["message_id", "user_id", "read_at"],
                "properties": {
                    "message_id": { "bsonType": "string" },
                    "user_id": { "bsonType": "string" },
                    "read_at": { "bsonType": "date" }
                }
            }
        };
        db.create_collection("read_receipts")
            .validator(receipts_schema)
            .await?;

        let rr_col = db.collection::<mongodb::bson::Document>("read_receipts");
        let idx_rr = IndexModel::builder()
            .keys(doc! { "message_id": 1, "user_id": 1 })
            .options(mongodb::options::IndexOptions::builder().unique(true).build())
            .build();
        rr_col.create_index(idx_rr).await?;

        // Index: message_id alone — for "who read this message"
        let idx_msg = IndexModel::builder()
            .keys(doc! { "message_id": 1 })
            .build();
        rr_col.create_index(idx_msg).await?;

        Ok(())
    }

    async fn down(&self, db: &Database) -> Result<(), MongoError> {
        db.collection::<mongodb::bson::Document>("messages").drop().await?;
        db.collection::<mongodb::bson::Document>("read_receipts").drop().await?;
        Ok(())
    }
}

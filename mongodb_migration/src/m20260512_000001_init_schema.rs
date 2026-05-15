use mongodb::{Client, Database, bson::doc, error::Error as MongoError};
use async_trait::async_trait;

#[async_trait]
pub trait MongoMigration {
    async fn up(&self, db: &Database) -> Result<(), MongoError>;
    async fn down(&self, db: &Database) -> Result<(), MongoError>;
    fn name(&self) -> &str;
}

pub struct InitSchemaMigration;

#[async_trait]
impl MongoMigration for InitSchemaMigration {
    fn name(&self) -> &str {
        "m20260512_000001_init_schema"
    }

    async fn up(&self, db: &Database) -> Result<(), MongoError> {
        // 1. Create `user_preferences` collection with schema validation
        let user_pref_schema = doc! {
            "$jsonSchema": {
                "bsonType": "object",
                "required": ["_id", "preferences"],
                "properties": {
                    "_id": {
                        "bsonType": "string",
                        "description": "must be a string (UUID) and is required"
                    },
                    "preferences": {
                        "bsonType": "array",
                        "description": "must be an array and is required",
                        "items": {
                            "bsonType": "object",
                            "required": ["category_id", "os_enabled"],
                            "properties": {
                                "category_id": { "bsonType": "int" },
                                "os_enabled": { "bsonType": "bool" }
                            }
                        }
                    }
                }
            }
        };

        db.create_collection("user_preferences")
            .validator(user_pref_schema)
            .await?;

        // 2. Create `notifications` collection (Bucket pattern) with schema validation
        let notif_bucket_schema = doc! {
            "$jsonSchema": {
                "bsonType": "object",
                "required": ["user_id", "page", "count", "notifications"],
                "properties": {
                    "user_id": {
                        "bsonType": "string",
                        "description": "must be a string (UUID) and is required"
                    },
                    "page": {
                        "bsonType": "int",
                        "description": "must be an integer and is required"
                    },
                    "count": {
                        "bsonType": "int",
                        "maximum": 30,
                        "description": "must be an integer <= 30 and is required"
                    },
                    "notifications": {
                        "bsonType": "array",
                        "maxItems": 30,
                        "description": "must be an array of max 30 items",
                        "items": {
                            "bsonType": "object",
                            "required": ["notification_id", "category_id", "title", "body", "is_read", "created_at"],
                            "properties": {
                                "notification_id": { "bsonType": "string" },
                                "category_id": { "bsonType": "int" },
                                "title": { "bsonType": "string" },
                                "body": { "bsonType": "string" },
                                "data": { "bsonType": "object" },
                                "is_read": { "bsonType": "bool" },
                                "os_notification_id": { "bsonType": ["string", "null"] },
                                "created_at": { "bsonType": "date" }
                            }
                        }
                    }
                }
            }
        };

        db.create_collection("notifications")
            .validator(notif_bucket_schema)
            .await?;

        // Create indexes for notifications
        let notif_col = db.collection::<mongodb::bson::Document>("notifications");
        let index_model = mongodb::IndexModel::builder()
            .keys(doc! { "user_id": 1, "page": -1 })
            .options(mongodb::options::IndexOptions::builder().unique(true).build())
            .build();
        notif_col.create_index(index_model).await?;

        Ok(())
    }

    async fn down(&self, db: &Database) -> Result<(), MongoError> {
        db.collection::<mongodb::bson::Document>("user_preferences").drop().await?;
        db.collection::<mongodb::bson::Document>("notifications").drop().await?;
        Ok(())
    }
}

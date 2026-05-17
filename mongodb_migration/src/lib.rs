pub mod m20260512_000001_init_schema;
pub mod m20260513_000001_add_notification_indexes;
pub mod m20260520_000001_init_chat_schema;

use mongodb::{Database, error::Error as MongoError};
use m20260512_000001_init_schema::{MongoMigration, InitSchemaMigration};
use m20260513_000001_add_notification_indexes::AddNotificationIndexes;
use m20260520_000001_init_chat_schema::InitChatSchema;

pub async fn run_migrations(db: &Database) -> Result<(), MongoError> {
    // 1. Initial schema creation
    let migration = InitSchemaMigration;
    println!("Running migration: {}", migration.name());
    migration.up(db).await?;
    println!("Migration {} completed.", migration.name());

    // 2. Add notification indexes
    let idx_migration = AddNotificationIndexes;
    println!("Running migration: {}", idx_migration.name());
    idx_migration.up(db).await?;
    println!("Migration {} completed.", idx_migration.name());

    // 3. Init chat schema
    let chat_migration = InitChatSchema;
    println!("Running migration: {}", chat_migration.name());
    chat_migration.up(db).await?;
    println!("Migration {} completed.", chat_migration.name());

    Ok(())
}

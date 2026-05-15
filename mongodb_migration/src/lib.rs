pub mod m20260512_000001_init_schema;
pub mod m20260513_000001_add_notification_indexes;

use mongodb::{Database, error::Error as MongoError};
use m20260512_000001_init_schema::{MongoMigration, InitSchemaMigration};
use m20260513_000001_add_notification_indexes::AddNotificationIndexes;

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

    Ok(())
}

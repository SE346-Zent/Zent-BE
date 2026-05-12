pub mod m20260512_000001_init_schema;

use mongodb::{Database, error::Error as MongoError};
use m20260512_000001_init_schema::{MongoMigration, InitSchemaMigration};

pub async fn run_migrations(db: &Database) -> Result<(), MongoError> {
    let migration = InitSchemaMigration;
    println!("Running migration: {}", migration.name());
    migration.up(db).await?;
    println!("Migration {} completed.", migration.name());
    Ok(())
}

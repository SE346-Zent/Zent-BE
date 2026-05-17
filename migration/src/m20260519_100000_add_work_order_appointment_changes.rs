use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        todo!("Migration m20260519_100000_add_work_order_appointment_changes has already been applied to the database — this file exists as a placeholder to satisfy the migrator.")
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        todo!("Migration m20260519_100000_add_work_order_appointment_changes has already been applied to the database — down migration not implemented.")
    }
}

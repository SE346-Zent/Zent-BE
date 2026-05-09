use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DbBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Change approver_id to be nullable
        if manager.get_database_backend() != DbBackend::Sqlite {
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderRejectForms::Table)
                        .modify_column(ColumnDef::new(WorkOrderRejectForms::ApproverId).uuid().null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert approver_id to be NOT NULL
        // Note: This might fail if there are existing NULL values in the database
        if manager.get_database_backend() != DbBackend::Sqlite {
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderRejectForms::Table)
                        .modify_column(
                            ColumnDef::new(WorkOrderRejectForms::ApproverId)
                                .uuid()
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrderRejectForms {
    Table,
    ApproverId,
}

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Change approver_id to be nullable
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .modify_column(ColumnDef::new(WorkOrderRejectForms::ApproverId).uuid().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Revert approver_id to be NOT NULL
        // Note: This might fail if there are existing NULL values in the database
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
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrderRejectForms {
    Table,
    ApproverId,
}

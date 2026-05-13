use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingForms::Table)
                    .rename_column(WorkOrderClosingForms::SignatureURL, WorkOrderClosingForms::SignatureFileName)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingForms::Table)
                    .rename_column(WorkOrderClosingForms::SignatureFileName, WorkOrderClosingForms::SignatureURL)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    SignatureURL,
    SignatureFileName,
}

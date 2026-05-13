use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite: one ALTER per statement.
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .add_column(ColumnDef::new(WorkOrderClosingImageLinks::Latitude).double().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .add_column(ColumnDef::new(WorkOrderClosingImageLinks::Longitude).double().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .add_column(
                        ColumnDef::new(WorkOrderClosingImageLinks::IsVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .drop_column(WorkOrderClosingImageLinks::Latitude)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .drop_column(WorkOrderClosingImageLinks::Longitude)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .drop_column(WorkOrderClosingImageLinks::IsVerified)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrderClosingImageLinks {
    Table,
    Latitude,
    Longitude,
    IsVerified,
}

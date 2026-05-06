use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add latitude, longitude, and is_verified to work_order_closing_image_links
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingImageLinks::Table)
                    .add_column(ColumnDef::new(WorkOrderClosingImageLinks::Latitude).double().null())
                    .add_column(ColumnDef::new(WorkOrderClosingImageLinks::Longitude).double().null())
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
                    .drop_column(WorkOrderClosingImageLinks::Longitude)
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

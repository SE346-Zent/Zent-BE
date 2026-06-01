use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename city to ward in work_orders table
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .rename_column(WorkOrders::City, WorkOrders::Ward)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename ward back to city
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .rename_column(WorkOrders::Ward, WorkOrders::City)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    City,
    Ward,
}

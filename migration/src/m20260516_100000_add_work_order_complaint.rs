use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .add_column(
                        ColumnDef::new(WorkOrders::CustomerComplaint)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(WorkOrders::CustomerComplaintAt)
                            .date_time()
                            .null(),
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
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::CustomerComplaintAt)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::CustomerComplaint)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    CustomerComplaint,
    CustomerComplaintAt,
}

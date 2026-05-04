use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add a Unique Index to work_order_number
        manager
            .create_index(
                Index::create()
                    .name("idx-work-order-number-unique")
                    .table(WorkOrders::Table)
                    .col(WorkOrders::WorkOrderNumber)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx-work-order-number-unique")
                    .table(WorkOrders::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    WorkOrderNumber,
}

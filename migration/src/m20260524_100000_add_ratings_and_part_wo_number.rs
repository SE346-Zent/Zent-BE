use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create table `work_order_ratings`
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderRatings::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderRatings::Id).primary_key())
                    .col(uuid(WorkOrderRatings::WorkOrderId).unique_key())
                    .col(integer(WorkOrderRatings::Rating))
                    .col(string_null(WorkOrderRatings::Comment))
                    .col(timestamp(WorkOrderRatings::CreatedAt))
                    .col(timestamp(WorkOrderRatings::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_ratings_work_order")
                            .from(WorkOrderRatings::Table, WorkOrderRatings::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Drop complaint columns in `work_orders`
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::CustomerComplaint)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::CustomerComplaintAt)
                    .to_owned(),
            )
            .await?;

        // 3. Add `work_order_number` to `new_part_forms`
        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .add_column(
                        string(NewPartForms::WorkOrderNumber)
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Remove `work_order_number` from `new_part_forms`
        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .drop_column(NewPartForms::WorkOrderNumber)
                    .to_owned(),
            )
            .await?;

        // 2. Re-add complaint columns to `work_orders`
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .add_column(
                        ColumnDef::new(WorkOrders::CustomerComplaint)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .add_column(
                        ColumnDef::new(WorkOrders::CustomerComplaintAt)
                            .date_time()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. Drop table `work_order_ratings`
        manager
            .drop_table(Table::drop().table(WorkOrderRatings::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrderRatings {
    Table,
    Id,
    WorkOrderId,
    Rating,
    Comment,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    Id,
    CustomerComplaint,
    CustomerComplaintAt,
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    WorkOrderNumber,
}

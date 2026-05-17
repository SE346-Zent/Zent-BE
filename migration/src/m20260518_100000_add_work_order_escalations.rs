use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the old escalation column from work_orders (revert previous approach)
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::EscalationLevelNotified)
                    .to_owned(),
            )
            .await?;

        // Create the new work_order_escalations audit table
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderEscalations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderEscalations::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::WorkOrderId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::EscalationLevel)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::ElapsedMinutes)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::BaselineMinutes)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::NotifiedSaCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::NotifiedAdminCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(WorkOrderEscalations::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_escalation_work_order")
                            .from(WorkOrderEscalations::Table, WorkOrderEscalations::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkOrderEscalations::Table).to_owned())
            .await?;

        // Re-add the old column
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .add_column(
                        ColumnDef::new(WorkOrders::EscalationLevelNotified)
                            .integer()
                            .null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    Id,
    EscalationLevelNotified,
}

#[derive(DeriveIden)]
enum WorkOrderEscalations {
    Table,
    Id,
    WorkOrderId,
    EscalationLevel,
    ElapsedMinutes,
    BaselineMinutes,
    NotifiedSaCount,
    NotifiedAdminCount,
    CreatedAt,
}

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create pause audit table
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderPauseActions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderPauseActions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderPauseActions::WorkOrderId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderPauseActions::PausedById)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderPauseActions::Reason)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderPauseActions::Explanation)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderPauseActions::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_pause_actions_work_order")
                            .from(WorkOrderPauseActions::Table, WorkOrderPauseActions::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_pause_actions_paused_by")
                            .from(WorkOrderPauseActions::Table, WorkOrderPauseActions::PausedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Insert "Paused" into work_order_statuses
        manager
            .exec_stmt(
                sea_query::Query::insert()
                    .into_table(WorkOrderStatuses::Table)
                    .columns([WorkOrderStatuses::Name])
                    .values_panic(["Paused".into()])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove "Paused" status
        manager
            .exec_stmt(
                sea_query::Query::delete()
                    .from_table(WorkOrderStatuses::Table)
                    .and_where(sea_query::Expr::col(WorkOrderStatuses::Name).eq("Paused"))
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(WorkOrderPauseActions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrderPauseActions {
    Table,
    Id,
    WorkOrderId,
    PausedById,
    Reason,
    Explanation,
    CreatedAt,
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum WorkOrderStatuses {
    Table,
    Name,
}

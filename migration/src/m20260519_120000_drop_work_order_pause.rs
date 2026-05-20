use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = manager.get_database_backend();

        // 1. Safely clear FK references before deleting the "Paused" status.
        //    Use raw SQL since SeaQuery can't express subquery-based updates.

        // 1a. Redirect state history rows pointing to "Paused" back to "InProg"
        //     (pauses only happen from InProg; to_status_id is NOT NULL so we can't set NULL)
        let sql_fix_history = match backend {
            sea_orm::DatabaseBackend::MySql => {
                "UPDATE work_order_state_history \
                 SET to_status_id = (SELECT id FROM work_order_statuses WHERE name = 'InProg' LIMIT 1) \
                 WHERE to_status_id IN (SELECT id FROM work_order_statuses WHERE name = 'Paused')"
            }
            sea_orm::DatabaseBackend::Postgres => {
                "UPDATE work_order_state_history \
                 SET to_status_id = (SELECT id FROM work_order_statuses WHERE name = 'InProg' LIMIT 1) \
                 WHERE to_status_id IN (SELECT id FROM work_order_statuses WHERE name = 'Paused')"
            }
            sea_orm::DatabaseBackend::Sqlite => {
                "UPDATE work_order_state_history \
                 SET to_status_id = (SELECT id FROM work_order_statuses WHERE name = 'InProg' LIMIT 1) \
                 WHERE to_status_id IN (SELECT id FROM work_order_statuses WHERE name = 'Paused')"
            }
        };
        db.execute_unprepared(sql_fix_history).await?;

        // 1b. Reset work orders in "Paused" state back to "Assigned"
        let sql_reset_work_orders = match backend {
            sea_orm::DatabaseBackend::MySql => {
                "UPDATE work_orders \
                 SET work_order_status_id = (SELECT id FROM work_order_statuses WHERE name = 'Assigned' LIMIT 1) \
                 WHERE work_order_status_id IN (SELECT id FROM work_order_statuses WHERE name = 'Paused')"
            }
            sea_orm::DatabaseBackend::Postgres => {
                "UPDATE work_orders \
                 SET work_order_status_id = (SELECT id FROM work_order_statuses WHERE name = 'Assigned' LIMIT 1) \
                 WHERE work_order_status_id IN (SELECT id FROM work_order_statuses WHERE name = 'Paused')"
            }
            sea_orm::DatabaseBackend::Sqlite => {
                "UPDATE work_orders \
                 SET work_order_status_id = (SELECT id FROM work_order_statuses WHERE name = 'Assigned' LIMIT 1) \
                 WHERE work_order_status_id IN (SELECT id FROM work_order_statuses WHERE name = 'Paused')"
            }
        };
        db.execute_unprepared(sql_reset_work_orders).await?;

        // 2. Remove "Paused" status from lookup table
        manager
            .exec_stmt(
                sea_query::Query::delete()
                    .from_table(WorkOrderStatuses::Table)
                    .and_where(sea_query::Expr::col(WorkOrderStatuses::Name).eq("Paused"))
                    .to_owned(),
            )
            .await?;

        // 3. Drop the pause audit log table
        manager
            .drop_table(
                Table::drop()
                    .table(WorkOrderPauseActions::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Re-create the pause audit table (in reverse order of up)
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

        // Re-insert "Paused" into work_order_statuses
        manager
            .exec_stmt(
                sea_query::Query::insert()
                    .into_table(WorkOrderStatuses::Table)
                    .columns([WorkOrderStatuses::Name])
                    .values_panic(["Paused".into()])
                    .to_owned(),
            )
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

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DbBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Drop the FK on work_order_status_id (ignore if not found)
        if manager.get_database_backend() != DbBackend::Sqlite {
            let _ = manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderStateHistory::Table)
                        .drop_foreign_key(Alias::new("work_order_state_history_ibfk_2"))
                        .to_owned(),
                )
                .await;
        }

        // Step 2: Rename work_order_status_id → to_status_id
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderStateHistory::Table)
                    .rename_column(
                        WorkOrderStateHistory::WorkOrderStatusId,
                        WorkOrderStateHistory::ToStatusId,
                    )
                    .to_owned(),
            )
            .await?;

        // Step 3: Add from_status_id (nullable)
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderStateHistory::Table)
                    .add_column(
                        ColumnDef::new(WorkOrderStateHistory::FromStatusId)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() != DbBackend::Sqlite {
            // Step 4: Add FK on to_status_id → work_order_statuses(id)
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderStateHistory::Table)
                        .add_foreign_key(
                            &TableForeignKey::new()
                                .name("fk_state_history_to_status")
                                .from_tbl(WorkOrderStateHistory::Table)
                                .from_col(WorkOrderStateHistory::ToStatusId)
                                .to_tbl(WorkOrderStatuses::Table)
                                .to_col(WorkOrderStatuses::Id)
                                .on_delete(ForeignKeyAction::Restrict)
                                .on_update(ForeignKeyAction::Cascade)
                                .to_owned(),
                        )
                        .to_owned(),
                )
                .await?;

            // Step 5: Add FK on from_status_id → work_order_statuses(id)
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderStateHistory::Table)
                        .add_foreign_key(
                            &TableForeignKey::new()
                                .name("fk_state_history_from_status")
                                .from_tbl(WorkOrderStateHistory::Table)
                                .from_col(WorkOrderStateHistory::FromStatusId)
                                .to_tbl(WorkOrderStatuses::Table)
                                .to_col(WorkOrderStatuses::Id)
                                .on_delete(ForeignKeyAction::Restrict)
                                .on_update(ForeignKeyAction::Cascade)
                                .to_owned(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Sqlite {
            // Drop new FKs
            let _ = manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderStateHistory::Table)
                        .drop_foreign_key(Alias::new("fk_state_history_to_status"))
                        .to_owned(),
                )
                .await;

            let _ = manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderStateHistory::Table)
                        .drop_foreign_key(Alias::new("fk_state_history_from_status"))
                        .to_owned(),
                )
                .await;
        }

        // Drop from_status_id
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderStateHistory::Table)
                    .drop_column(WorkOrderStateHistory::FromStatusId)
                    .to_owned(),
            )
            .await?;

        // Rename to_status_id → work_order_status_id
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderStateHistory::Table)
                    .rename_column(
                        WorkOrderStateHistory::ToStatusId,
                        WorkOrderStateHistory::WorkOrderStatusId,
                    )
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() != DbBackend::Sqlite {
            // Restore FK on work_order_status_id
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkOrderStateHistory::Table)
                        .add_foreign_key(
                            &TableForeignKey::new()
                                .name("work_order_state_history_ibfk_2")
                                .from_tbl(WorkOrderStateHistory::Table)
                                .from_col(WorkOrderStateHistory::WorkOrderStatusId)
                                .to_tbl(WorkOrderStatuses::Table)
                                .to_col(WorkOrderStatuses::Id)
                                .on_delete(ForeignKeyAction::Restrict)
                                .on_update(ForeignKeyAction::Cascade)
                                .to_owned(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrderStateHistory {
    Table,
    WorkOrderStatusId,
    FromStatusId,
    ToStatusId,
}

#[derive(DeriveIden)]
enum WorkOrderStatuses {
    Table,
    Id,
}

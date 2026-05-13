use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite: one ALTER per statement
        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .add_column(ColumnDef::new(NewPartForms::WorkOrderId).uuid().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_new_part_forms_work_order")
                            .from_tbl(NewPartForms::Table)
                            .from_col(NewPartForms::WorkOrderId)
                            .to_tbl(WorkOrders::Table)
                            .to_col(WorkOrders::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Make reason and explanation NOT NULL in work_order_reject_forms
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .modify_column(ColumnDef::new(WorkOrderRejectForms::Reason).string().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .modify_column(ColumnDef::new(WorkOrderRejectForms::Explanation).string().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .drop_foreign_key(Alias::new("fk_new_part_forms_work_order"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .drop_column(NewPartForms::WorkOrderId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .modify_column(ColumnDef::new(WorkOrderRejectForms::Reason).string().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .modify_column(ColumnDef::new(WorkOrderRejectForms::Explanation).string().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    WorkOrderId,
}

#[derive(DeriveIden)]
enum WorkOrderRejectForms {
    Table,
    Reason,
    Explanation,
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    Id,
}

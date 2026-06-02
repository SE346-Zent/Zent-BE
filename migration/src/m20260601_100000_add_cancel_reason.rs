use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CancelReasons::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CancelReasons::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(CancelReasons::WorkOrderId).uuid().not_null())
                    .col(ColumnDef::new(CancelReasons::CancelledBy).uuid().not_null())
                    .col(ColumnDef::new(CancelReasons::Reason).string_len(1000).not_null())
                    .col(ColumnDef::new(CancelReasons::AdditionalComments).text().null())
                    .col(ColumnDef::new(CancelReasons::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancel_reasons_work_order")
                            .from(CancelReasons::Table, CancelReasons::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cancel_reasons_user")
                            .from(CancelReasons::Table, CancelReasons::CancelledBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CancelReasons::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CancelReasons {
    Table,
    Id,
    WorkOrderId,
    CancelledBy,
    Reason,
    AdditionalComments,
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

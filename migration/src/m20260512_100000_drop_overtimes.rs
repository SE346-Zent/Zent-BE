use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Overtimes::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Overtimes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Overtimes::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Overtimes::TechnicianId).uuid().not_null())
                    .col(ColumnDef::new(Overtimes::WorkOrderId).uuid().not_null())
                    .col(ColumnDef::new(Overtimes::OvertimeMinutes).integer().not_null())
                    .col(ColumnDef::new(Overtimes::CreatedAt).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_overtimes_technician_id")
                            .from(Overtimes::Table, Overtimes::TechnicianId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_overtimes_work_order_id")
                            .from(Overtimes::Table, Overtimes::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Overtimes {
    Table,
    Id,
    TechnicianId,
    WorkOrderId,
    OvertimeMinutes,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    Id,
}

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderAppointmentChanges::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkOrderAppointmentChanges::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderAppointmentChanges::WorkOrderId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderAppointmentChanges::OldAppointment)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderAppointmentChanges::NewAppointment)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderAppointmentChanges::ChangedById)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkOrderAppointmentChanges::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_appointment_changes_work_order")
                            .from(WorkOrderAppointmentChanges::Table, WorkOrderAppointmentChanges::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_appointment_changes_changed_by")
                            .from(WorkOrderAppointmentChanges::Table, WorkOrderAppointmentChanges::ChangedById)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkOrderAppointmentChanges::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrderAppointmentChanges {
    Table,
    Id,
    WorkOrderId,
    OldAppointment,
    NewAppointment,
    ChangedById,
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

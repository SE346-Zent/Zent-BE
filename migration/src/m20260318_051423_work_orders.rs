use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkOrder::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrder::Id).primary_key())
                    .col(string(WorkOrder::Title))
                    .col(string(WorkOrder::AddressString))
                    .col(integer(WorkOrder::StatusId))
                    .col(string(WorkOrder::Description))
                    .col(string(WorkOrder::RejectReason))
                    .col(integer(WorkOrder::Priority))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(WorkOrder::ClosedAt))
                    .col(integer(WorkOrder::Version))
                    .col(uuid(WorkOrder::AdminId))
                    .col(uuid(WorkOrder::CustomerId))
                    .col(uuid(WorkOrder::TechnicianId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_status_id")
                            .from(WorkOrder::Table, WorkOrder::StatusId)
                            .to(WorkOrderStatus::Table, WorkOrderStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderStatus::Table)
                    .if_not_exists()
                    .col(pk_auto(WorkOrderStatus::Id))
                    .col(string(WorkOrderStatus::Name))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkOrder::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderStatus::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

#[derive(DeriveIden)]
enum WorkOrder {
    Table,
    Id,
    Title,
    Description,
    AddressString,
    StatusId,
    RejectReason,
    Priority,
    ClosedAt,
    Version,
    AdminId,
    CustomerId,
    TechnicianId,
}

#[derive(DeriveIden)]
enum WorkOrderStatus {
    Table,
    Id,
    Name,
}

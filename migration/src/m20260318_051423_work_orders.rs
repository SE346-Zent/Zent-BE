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
                    .col(string(WorkOrder::AddressString))
                    .col(uuid(WorkOrder::StatusId))
                    .col(string(WorkOrder::Description))
                    .col(string(WorkOrder::RejectReason))
                    .col(integer(WorkOrder::Priority))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(WorkOrder::ClosedAt))
                    .col(integer(WorkOrder::Version))
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
        
        manager.
            create_table(
                Table::create()
                    .table(WorkOrderStatus::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderStatus::Id).primary_key())
                    .col(string(WorkOrderStatus::Name))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!();

    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

#[derive(DeriveIden)]
struct DeletedAt;

#[derive(DeriveIden)]
enum WorkOrder
{
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
}

#[derive(DeriveIden)]
enum WorkOrderStatus
{
    Table,
    Id,
    Name,
}


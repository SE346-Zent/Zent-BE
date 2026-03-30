use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderClosingForms::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderClosingForms::Id).primary_key())
                    .col(string(WorkOrderClosingForms::WorkOrderCounting))
                    .col(string(WorkOrderClosingForms::MTM))
                    .col(string(WorkOrderClosingForms::SerialNumber))
                    .col(string(WorkOrderClosingForms::Diagnosis))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrder::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrder::Id).primary_key())
                    .col(string(WorkOrder::FirstName))
                    .col(string(WorkOrder::LastName))
                    .col(string(WorkOrder::Email))
                    .col(string(WorkOrder::PhoneNumber))
                    .col(string(WorkOrder::Role))
                    .col(integer(WorkOrder::WorkOrderStatusId))
                    .col(string(WorkOrder::Country))
                    .col(string(WorkOrder::State))
                    .col(string(WorkOrder::City))
                    .col(string(WorkOrder::Address))
                    .col(string(WorkOrder::Building))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp(ClosedAt))
                    .col(uuid(WorkOrder::AdminId))
                    .col(uuid(WorkOrder::CustomerId))
                    .col(uuid(WorkOrder::TechnicianId))
                    .col(uuid(WorkOrder::CompleteFormId))
                    .col(uuid(WorkOrder::RejectFormId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_status")
                            .from(WorkOrder::Table, WorkOrder::WorkOrderStatusId)
                            .to(WorkOrderStatus::Table, WorkOrderStatus::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_admin")
                            .from(WorkOrder::Table, WorkOrder::AdminId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_customer")
                            .from(WorkOrder::Table, WorkOrder::CustomerId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_technician")
                            .from(WorkOrder::Table, WorkOrder::TechnicianId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_complete_form")
                            .from(WorkOrder::Table, WorkOrder::CompleteFormId)
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_reject_form")
                            .from(WorkOrder::Table, WorkOrder::RejectFormId)
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
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

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderSymptoms::Table)
                    .if_not_exists()
                    .col(pk_auto(WorkOrderSymptoms::Id))
                    .col(string(WorkOrderSymptoms::SymptomNames))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkOrderClosingForms::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderStatus::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderSymptoms::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrder::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;
#[derive(DeriveIden)]
struct ClosedAt;

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    Id,
    WorkOrderCounting,
    MTM,
    SerialNumber,
    Diagnosis,
}

// TODO: reject form ID

#[derive(DeriveIden)]
enum WorkOrder
{
    Table,
    Id,
    CompleteFormId,
    RejectFormId,
    CustomerId,
    TechnicianId,
    AdminId,
    WorkOrderStatusId,
    FirstName,
    LastName,
    Email,
    PhoneNumber,
    Role,
    Country,
    State,
    City,
    Address,
    Building
}

#[derive(DeriveIden)]
enum WorkOrderSymptoms {
    Table,
    Id,
    SymptomNames
}


#[derive(DeriveIden)]
enum WorkOrderStatus {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    FullName,
    Email,
    PasswordHash,
    PhoneNumber,
    AccountStatus,
    RoleID,
}
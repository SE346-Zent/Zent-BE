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
                    .table(WorkOrders::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrders::Id).primary_key())
                    .col(string(WorkOrders::FirstName))
                    .col(string(WorkOrders::LastName))
                    .col(string(WorkOrders::Email))
                    .col(string(WorkOrders::PhoneNumber))
                    .col(integer(WorkOrders::WorkOrderStatusId))
                    .col(string(WorkOrders::Country))
                    .col(string(WorkOrders::State))
                    .col(string(WorkOrders::City))
                    .col(string(WorkOrders::Address))
                    .col(string(WorkOrders::Building))
                    .col(timestamp(WorkOrders::Appointment))
                    .col(string(WorkOrders::ReferenceTicket))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp(ClosedAt))
                    .col(uuid(WorkOrders::AdminId))
                    .col(uuid(WorkOrders::CustomerId))
                    .col(uuid(WorkOrders::TechnicianId))
                    .col(uuid(WorkOrders::CompleteFormId))
                    .col(uuid(WorkOrders::RejectFormId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_status")
                            .from(WorkOrders::Table, WorkOrders::WorkOrderStatusId)
                            .to(WorkOrderStatus::Table, WorkOrderStatus::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_admin")
                            .from(WorkOrders::Table, WorkOrders::AdminId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_customer")
                            .from(WorkOrders::Table, WorkOrders::CustomerId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_technician")
                            .from(WorkOrders::Table, WorkOrders::TechnicianId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_complete_form")
                            .from(WorkOrders::Table, WorkOrders::CompleteFormId)
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_reject_form")
                            .from(WorkOrders::Table, WorkOrders::RejectFormId)
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
            .drop_table(Table::drop().table(WorkOrders::Table).to_owned())
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
enum WorkOrders
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
    Building,
    Appointment,
    ReferenceTicket,
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
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
                    .col(string(WorkOrderClosingForms::WorkOrderNumber))
                    .col(string(WorkOrderClosingForms::MTM))
                    .col(string(WorkOrderClosingForms::SerialNumber))
                    .col(uuid(WorkOrderClosingForms::WorkOrderId))
                    .col(string(WorkOrderClosingForms::Diagnosis))
                    .col(string(WorkOrderClosingForms::SignatureUrl))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_closing_forms_product_models")
                            .from(WorkOrderClosingForms::Table, WorkOrderClosingForms::MTM)
                            .to(ProductModels::Table, ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_closing_forms_work_orders")
                            .from(WorkOrderClosingForms::Table, WorkOrderClosingForms::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RejectionForm::Table)
                    .if_not_exists()
                    .col(uuid(RejectionForm::RejectFormId).primary_key())
                    .col(string(RejectionForm::RejectReason))
                    .col(uuid(RejectionForm::ApproverId))
                    .col(boolean(RejectionForm::Approved))
                    .col(string_null(RejectionForm::ReviewReason))
                    .col(uuid(RejectionForm::WorkOrderId))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rejection_form_work_order")
                            .from(RejectionForm::Table, RejectionForm::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rejection_form_approver")
                            .from(RejectionForm::Table, RejectionForm::ApproverId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
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
                    .col(string_null(WorkOrders::Email))
                    .col(string_null(WorkOrders::PhoneNumber))
                    .col(integer(WorkOrders::WorkOrderStatusId))
                    .col(string(WorkOrders::Country))
                    .col(string(WorkOrders::State))
                    .col(string(WorkOrders::City))
                    .col(string(WorkOrders::Address))
                    .col(string_null(WorkOrders::Building))
                    .col(timestamp(WorkOrders::Appointment))
                    .col(uuid_null(WorkOrders::ReferenceTicket))
                    .col(string(WorkOrders::Description))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .col(uuid(WorkOrders::AssignerAdminId))
                    .col(uuid(WorkOrders::CustomerId))
                    .col(uuid_null(WorkOrders::AssignedTechnicianId))
                    .col(uuid_null(WorkOrders::CompleteFormId))
                    .col(string_null(WorkOrders::RejectReason))
                    .col(integer(WorkOrders::WorkOrderSymptomId))
                    .col(uuid(WorkOrders::ProductId))
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
                            .from(WorkOrders::Table, WorkOrders::AssignerAdminId)
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
                            .from(WorkOrders::Table, WorkOrders::AssignedTechnicianId)
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
                            .name("fk_work_order_symptom")
                            .from(WorkOrders::Table, WorkOrders::WorkOrderSymptomId)
                            .to(WorkOrderSymptoms::Table, WorkOrderSymptoms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_product")
                            .from(WorkOrders::Table, WorkOrders::ProductId)
                            .to(Products::Table, Products::Id)
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
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
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
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
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
struct DeletedAt;

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    Id,
    WorkOrderNumber,
    WorkOrderId,
    MTM,
    SerialNumber,
    Diagnosis,
    SignatureUrl
}

#[derive(DeriveIden)]
enum Products
{
    Table,
    Id, 
}

#[derive(DeriveIden)]
enum WorkOrders
{
    Table,
    Id,
    CompleteFormId,
    RejectReason,
    CustomerId,
    AssignedTechnicianId,
    AssignerAdminId,
    WorkOrderStatusId,
    WorkOrderSymptomId,
    ProductId,
    FirstName,
    LastName,
    Email,
    PhoneNumber,
    Country,
    State,
    City,
    Address,
    Building,
    Appointment,
    ReferenceTicket,
    Description,
}

#[derive(DeriveIden)]
enum RejectionForm
{
    Table,
    RejectFormId,
    RejectReason,
    ApproverId,
    Approved,
    ReviewReason,
    WorkOrderId,
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
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}
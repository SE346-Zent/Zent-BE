use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderClosingForms::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderClosingForms::Id).primary_key())
                    .col(string(WorkOrderClosingForms::MTM))
                    .col(uuid(WorkOrderClosingForms::WorkOrderId))
                    .col(string(WorkOrderClosingForms::SerialNumber))
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
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RejectionForm::Table)
                    .if_not_exists()
                    .col(uuid(RejectionForm::RejectFormId).primary_key())
                    .col(uuid(RejectionForm::ApproverId))
                    .col(uuid(RejectionForm::WorkOrderId))
                    .col(string(RejectionForm::RejectReason))
                    .col(boolean(RejectionForm::Approved))
                    .col(string_null(RejectionForm::ReviewReason))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
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
                    .col(integer(WorkOrders::WorkOrderStatusId))
                    .col(uuid(WorkOrders::AssignerAdminId))
                    .col(uuid(WorkOrders::CustomerId))
                    .col(uuid_null(WorkOrders::AssignedTechnicianId))
                    .col(uuid_null(WorkOrders::CompleteFormId))
                    .col(uuid_null(WorkOrders::RejectFormId))
                    .col(integer(WorkOrders::WorkOrderSymptomId))
                    .col(uuid(WorkOrders::ProductId))
                    .col(uuid_null(WorkOrders::ReferenceTicket))
                    .col(string(WorkOrders::WorkOrderNumber))
                    .col(string(WorkOrders::FirstName))
                    .col(string(WorkOrders::LastName))
                    .col(string_null(WorkOrders::Email))
                    .col(string_null(WorkOrders::PhoneNumber))
                    .col(string(WorkOrders::Country))
                    .col(string(WorkOrders::State))
                    .col(string(WorkOrders::City))
                    .col(string(WorkOrders::Address))
                    .col(string_null(WorkOrders::Building))
                    .col(timestamp(WorkOrders::Appointment))
                    .col(string(WorkOrders::Description))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
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
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_reject_form")
                            .from(WorkOrders::Table, WorkOrders::RejectFormId)
                            .to(RejectionForm::Table, RejectionForm::RejectFormId)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        // Now add the FK from WorkOrderClosingForms -> WorkOrders
        // (WorkOrderClosingForms was created before WorkOrders, so the FK
        //  to work_orders could not be declared inline. We add it here.)
        // NOTE: SQLite does not support ALTER TABLE ADD FOREIGN KEY.
        // The WorkOrderClosingForms.WorkOrderId -> WorkOrders.Id relationship
        // is enforced at the application/ORM level for SQLite.
        // The RejectionForm.WorkOrderId -> WorkOrders.Id relationship
        // is also enforced at the application/ORM level for SQLite.

        // AddPartRequestImages table
        manager
            .create_table(
                Table::create()
                    .table(AddPartRequestImages::Table)
                    .if_not_exists()
                    .col(uuid(AddPartRequestImages::Id).primary_key())
                    .col(uuid(AddPartRequestImages::AddPartRequestId))
                    .col(string(AddPartRequestImages::ImageURL))
                    .col(timestamp(AddPartRequestImages::CapturedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_add_part_request_images_add_new_part_form")
                            .from(AddPartRequestImages::Table, AddPartRequestImages::AddPartRequestId)
                            .to(AddNewPartForm::Table, AddNewPartForm::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        // WorkOrderImages table
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderImages::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderImages::Id).primary_key())
                    .col(uuid(WorkOrderImages::WorkOrderId))
                    .col(string(WorkOrderImages::ImageURL))
                    .col(timestamp(WorkOrderImages::CapturedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_images_work_order")
                            .from(WorkOrderImages::Table, WorkOrderImages::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        // WorkOrderClosingFormImages table
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderClosingFormImages::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderClosingFormImages::Id).primary_key())
                    .col(uuid(WorkOrderClosingFormImages::WorkOrderClosingFormId))
                    .col(string(WorkOrderClosingFormImages::ImageURL))
                    .col(timestamp(WorkOrderClosingFormImages::CapturedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_closing_form_images_form")
                            .from(WorkOrderClosingFormImages::Table, WorkOrderClosingFormImages::WorkOrderClosingFormId)
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse dependency order
        manager
            .drop_table(Table::drop().table(WorkOrderClosingFormImages::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderImages::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AddPartRequestImages::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrders::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RejectionForm::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderClosingForms::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderStatus::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WorkOrderSymptoms::Table).to_owned())
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
    MTM,
    WorkOrderId,
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
    WorkOrderStatusId,
    AssignerAdminId,
    CustomerId,
    AssignedTechnicianId,
    CompleteFormId,
    RejectFormId,
    WorkOrderSymptomId,
    ProductId,
    ReferenceTicket,
    WorkOrderNumber,
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
    Description,
}

#[derive(DeriveIden)]
enum RejectionForm
{
    Table,
    RejectFormId,
    ApproverId,
    WorkOrderId,
    RejectReason,
    Approved,
    ReviewReason,
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

#[derive(DeriveIden)]
enum AddPartRequestImages {
    Table,
    Id,
    AddPartRequestId,
    ImageURL,
    CapturedAt,
}

#[derive(DeriveIden)]
enum WorkOrderImages {
    Table,
    Id,
    WorkOrderId,
    ImageURL,
    CapturedAt,
}

#[derive(DeriveIden)]
enum WorkOrderClosingFormImages {
    Table,
    Id,
    WorkOrderClosingFormId,
    ImageURL,
    CapturedAt,
}

#[derive(DeriveIden)]
enum AddNewPartForm
{
    Table,
    Id,
}
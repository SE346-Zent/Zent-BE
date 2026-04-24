use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkOrderStatuses::Table)
                    .if_not_exists()
                    .col(pk_auto(WorkOrderStatuses::Id))
                    .col(string(WorkOrderStatuses::Name))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderSymptoms::Table)
                    .if_not_exists()
                    .col(pk_auto(WorkOrderSymptoms::Id))
                    .col(string(WorkOrderSymptoms::Name))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderRejectForms::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderRejectForms::Id).primary_key())
                    .col(uuid(WorkOrderRejectForms::ApproverId))
                    .col(boolean(WorkOrderRejectForms::Approved))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_reject_forms_approver")
                            .from(WorkOrderRejectForms::Table, WorkOrderRejectForms::ApproverId)
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
                    .col(uuid(WorkOrders::CustomerId))
                    .col(uuid(WorkOrders::ProductId))
                    .col(uuid_null(WorkOrders::ReferenceTicketId))
                    .col(integer(WorkOrders::WorkOrderSymptomId))
                    .col(string(WorkOrders::Description))
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
                    .col(uuid(WorkOrders::AdminId))
                    .col(uuid_null(WorkOrders::TechnicianId))
                    .col(uuid_null(WorkOrders::CompleteFormId))
                    .col(string(WorkOrders::WorkOrderNumber))
                    .col(uuid_null(WorkOrders::RejectFormId))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_status")
                            .from(WorkOrders::Table, WorkOrders::WorkOrderStatusId)
                            .to(WorkOrderStatuses::Table, WorkOrderStatuses::Id)
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
                            .name("fk_work_order_product")
                            .from(WorkOrders::Table, WorkOrders::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_work_order_reference_ticket")
                            .from(WorkOrders::Table, WorkOrders::ReferenceTicketId)
                            .to(WorkOrders::Table, WorkOrders::Id)
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
                            .name("fk_work_order_admin")
                            .from(WorkOrders::Table, WorkOrders::AdminId)
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
                            .name("fk_work_order_reject_form")
                            .from(WorkOrders::Table, WorkOrders::RejectFormId)
                            .to(WorkOrderRejectForms::Table, WorkOrderRejectForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderClosingForms::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderClosingForms::Id).primary_key())
                    .col(uuid(WorkOrderClosingForms::ProductId))
                    .col(uuid(WorkOrderClosingForms::WorkOrderId).unique_key())
                    .col(string(WorkOrderClosingForms::Mtm))
                    .col(string(WorkOrderClosingForms::SerialNumber))
                    .col(string(WorkOrderClosingForms::Diagnosis))
                    .col(string(WorkOrderClosingForms::SignatureURL))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_closing_forms_product")
                            .from(WorkOrderClosingForms::Table, WorkOrderClosingForms::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_closing_forms_work_order")
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
                    .table(WorkOrderStateHistory::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderStateHistory::Id).primary_key())
                    .col(uuid(WorkOrderStateHistory::WorkOrderId))
                    .col(integer(WorkOrderStateHistory::WorkOrderStatusId))
                    .col(uuid(WorkOrderStateHistory::ChangedById))
                    .col(timestamp(WorkOrderStateHistory::ChangedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_state_history_work_order")
                            .from(WorkOrderStateHistory::Table, WorkOrderStateHistory::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_state_history_wo_status")
                            .from(WorkOrderStateHistory::Table, WorkOrderStateHistory::WorkOrderStatusId)
                            .to(WorkOrderStatuses::Table, WorkOrderStatuses::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_state_history_changed_by")
                            .from(WorkOrderStateHistory::Table, WorkOrderStateHistory::ChangedById)
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
                    .table(Images::Table)
                    .if_not_exists()
                    .col(uuid(Images::Id).primary_key())
                    .col(string(Images::ImageURL))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderImageLinks::ImageId))
                    .col(uuid(WorkOrderImageLinks::WorkOrderId))
                    .primary_key(
                        Index::create()
                            .col(WorkOrderImageLinks::ImageId)
                            .col(WorkOrderImageLinks::WorkOrderId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_image_links_image")
                            .from(WorkOrderImageLinks::Table, WorkOrderImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_wo_image_links_work_order")
                            .from(WorkOrderImageLinks::Table, WorkOrderImageLinks::WorkOrderId)
                            .to(WorkOrders::Table, WorkOrders::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ClosingFormImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(ClosingFormImageLinks::ImageId))
                    .col(uuid(ClosingFormImageLinks::WorkOrderClosingFormId))
                    .primary_key(
                        Index::create()
                            .col(ClosingFormImageLinks::ImageId)
                            .col(ClosingFormImageLinks::WorkOrderClosingFormId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cf_image_links_image")
                            .from(ClosingFormImageLinks::Table, ClosingFormImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cf_image_links_cf")
                            .from(ClosingFormImageLinks::Table, ClosingFormImageLinks::WorkOrderClosingFormId)
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;
        

        manager
            .create_table(
                Table::create()
                    .table(NewPartFormImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(NewPartFormImageLinks::ImageId))
                    .col(uuid(NewPartFormImageLinks::NewPartFormId))
                    .primary_key(
                        Index::create()
                            .col(NewPartFormImageLinks::ImageId)
                            .col(NewPartFormImageLinks::NewPartFormId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_npf_image_links_image")
                            .from(NewPartFormImageLinks::Table, NewPartFormImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_npf_image_links_npf")
                            .from(NewPartFormImageLinks::Table, NewPartFormImageLinks::NewPartFormId)
                            .to(NewPartForms::Table, NewPartForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse dependency order
        manager.drop_table(Table::drop().table(NewPartFormImageLinks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ClosingFormImageLinks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(WorkOrderImageLinks::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Images::Table).to_owned()).await?;
        
        manager.drop_table(Table::drop().table(WorkOrderStateHistory::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(WorkOrderClosingForms::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(WorkOrders::Table).to_owned()).await?;
        
        manager.drop_table(Table::drop().table(WorkOrderRejectForms::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(WorkOrderSymptoms::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(WorkOrderStatuses::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

#[derive(DeriveIden)]
struct DeletedAt;

#[derive(DeriveIden)]
enum WorkOrderStatuses {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum WorkOrderSymptoms {
    Table,
    Id,
    Name
}

#[derive(DeriveIden)]
enum WorkOrders
{
    Table,
    Id,
    WorkOrderNumber,
    WorkOrderStatusId,
    CustomerId,
    ProductId,
    ReferenceTicketId,
    WorkOrderSymptomId,
    Description,
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
    AdminId,
    TechnicianId,
    CompleteFormId,
    RejectFormId,
}

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    Id,
    ProductId,
    WorkOrderId,
    Mtm,
    SerialNumber,
    Diagnosis,
    SignatureURL,
}


#[derive(DeriveIden)]
enum WorkOrderRejectForms
{
    Table,
    Id,
    ApproverId,
    Approved,
}

#[derive(DeriveIden)]
enum WorkOrderStateHistory
{
    Table,
    Id,
    WorkOrderId,
    WorkOrderStatusId,
    ChangedById,
    ChangedAt,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
    ImageURL,
}

#[derive(DeriveIden)]
enum WorkOrderImageLinks {
    Table,
    ImageId,
    WorkOrderId,
}

#[derive(DeriveIden)]
enum ClosingFormImageLinks {
    Table,
    ImageId,
    WorkOrderClosingFormId,
}

#[derive(DeriveIden)]
enum NewPartFormImageLinks {
    Table,
    ImageId,
    NewPartFormId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    Id,
}
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProductModels::Table)
                    .if_not_exists()
                    .col(string(ProductModels::ModelCode).primary_key())
                    .col(string(ProductModels::ModelName))
                    .col(string_null(ProductModels::Description))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartMfgStatuses::Table)
                    .if_not_exists()
                    .col(pk_auto(PartMfgStatuses::Id))
                    .col(string(PartMfgStatuses::Name))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartTypes::Table)
                    .if_not_exists()
                    .col(string(PartTypes::PartTypeName).primary_key())
                    .col(string_null(PartTypes::Description))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Products::Table)
                    .if_not_exists()
                    .col(uuid(Products::Id).primary_key())
                    .col(string(Products::ProductModelCode))
                    .col(uuid(Products::CustomerId))
                    .col(string(Products::ProductName))
                    .col(string(Products::SerialNumber))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_products_model")
                            .from(Products::Table, Products::ProductModelCode)
                            .to(ProductModels::Table, ProductModels::ModelCode)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_products_customer")
                            .from(Products::Table, Products::CustomerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Warranties::Table)
                    .if_not_exists()
                    .col(uuid(Warranties::Id).primary_key())
                    .col(uuid(Warranties::CustomerId))
                    .col(uuid(Warranties::ProductId))
                    .col(timestamp(Warranties::StartDate))
                    .col(timestamp(Warranties::EndDate))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_warranty_product")
                            .from(Warranties::Table, Warranties::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_warranty_customer")
                            .from(Warranties::Table, Warranties::CustomerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NewPartForms::Table)
                    .if_not_exists()
                    .col(uuid(NewPartForms::Id).primary_key())
                    .col(string(NewPartForms::PartNumber))
                    .col(integer(NewPartForms::PartTypeId))
                    .col(string_null(NewPartForms::ModelCode))
                    .col(string(NewPartForms::SerialNumber))
                    .col(string_null(NewPartForms::Description))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_new_part_form_part_type")
                            .from(NewPartForms::Table, NewPartForms::PartTypeId)
                            .to(PartTypes::Table, PartTypes::PartTypeName)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_new_part_form_product_model")
                            .from(NewPartForms::Table, NewPartForms::ModelCode)
                            .to(ProductModels::Table, ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(NewPartForms::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Warranties::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Products::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartTypes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartMfgStatuses::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ProductModels::Table).to_owned()).await?;

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
enum Products
{
    Table,
    Id,
    ProductModelCode,
    CustomerId,
    ProductName,
    SerialNumber,   
}

#[derive(DeriveIden)]
enum ProductModels
{
    Table,
    ModelName,
    ModelCode,
    Description
}

#[derive(DeriveIden)]
enum PartMfgStatuses
{
    Table,
    Id,
    Name    
}

#[derive(DeriveIden)]
enum PartTypes {
    Table,
    PartTypeName,
    Description,
}

#[derive(DeriveIden)]
enum Warranties
{
    Table,
    Id,
    CustomerId,
    ProductId,
    StartDate,
    EndDate,
}

#[derive(DeriveIden)]
enum NewPartForms
{
    Table,
    Id,
    PartNumber,
    PartTypeId,
    ModelCode,
    SerialNumber,
    Description,
}

#[derive(DeriveIden)]
enum Users
{
    Table,
    Id
}

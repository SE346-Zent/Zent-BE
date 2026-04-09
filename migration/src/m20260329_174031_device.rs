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
                    .table(PartStatus::Table)
                    .if_not_exists()
                    .col(pk_auto(PartStatus::Id))
                    .col(string(PartStatus::Name))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartTypes::Table)
                    .if_not_exists()
                    .col(string(PartTypes::PartNumber).primary_key())
                    .col(string(PartTypes::CommodityType))
                    .col(string(PartTypes::Description))
                    .col(integer(PartTypes::PartMfgStatusId))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Products::Table)
                    .if_not_exists()
                    .col(uuid(Products::Id).primary_key())
                    .col(string(Products::ModelId))
                    .col(uuid(Products::CustomerId))
                    .col(string(Products::SerialNumber))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_products_model")
                            .from(Products::Table, Products::ModelId)
                            .to(ProductModels::Table, ProductModels::ModelCode)
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
                    .col(timestamp_null(Warranties::EndDate))
                    .col(string(Warranties::WarrantyStatus))
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
                    .col(uuid_null(Images::PartId))
                    .col(uuid_null(Images::ProductId))
                    .col(timestamp(Images::CapturedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_part")
                            .from(Images::Table, Images::PartId)
                            .to(PartTypes::Table, PartTypes::PartNumber)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_product")
                            .from(Images::Table, Images::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Images::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Warranties::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Products::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartTypes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartStatus::Table).to_owned()).await?;
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
    ModelId,
    CustomerId,
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
enum PartStatus
{
    Table,
    Id,
    Name    
}

#[derive(DeriveIden)]
enum PartTypes {
    Table,
    PartNumber,
    CommodityType,
    Description,
    PartMfgStatusId
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
    WarrantyStatus
}

#[derive(DeriveIden)]
enum Images
{
    Table,
    Id,
    ImageURL,
    PartId,
    ProductId,
    CapturedAt
}




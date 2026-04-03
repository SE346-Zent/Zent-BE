use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PartTypes::Table)
                    .if_not_exists()
                    .col(string(PartTypes::PartNumber).primary_key())
                    .col(string(PartTypes::CommodityType))
                    .col(string(PartTypes::Description))
                    .to_owned(),
                    
            )
            .await?;

        manager
            .create_table(
                Table::create()
                .table(PartInstallations::Table)
                .if_not_exists()
                .col(uuid(PartInstallations::Id).primary_key())
                .col(uuid(PartInstallations::ProductId))
                .col(string(PartInstallations::PartNumber))
                .col(integer(PartInstallations::Quantity))
                .foreign_key(
                        ForeignKey::create()
                        .name("fk_part_installation_part_type")
                        .from(PartInstallations::Table, PartInstallations::PartNumber)
                        .to(PartTypes::Table, PartTypes::PartNumber)
                        .on_update(ForeignKeyAction::Cascade)
                        .on_delete(ForeignKeyAction::Restrict),

                    )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_part_installation_product")
                    .from(PartInstallations::Table, PartInstallations::ProductId)
                    .to(Products::Table, Products::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                )
                .to_owned()   
            )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .drop_table(Table::drop().table(PartTypes::Table).to_owned())
            .await?;

        
        manager
            .drop_table(Table::drop().table(PartInstallations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PartTypes {
    Table,
    PartNumber,
    CommodityType,
    Description
}

#[derive(DeriveIden)]
enum PartInstallations 
{
    Table,
    Id,
    PartNumber,
    ProductId,
    Quantity,
}

#[derive(DeriveIden)]
enum Products
{
    Table,
    Id,
}

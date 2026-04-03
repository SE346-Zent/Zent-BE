use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        manager
            .create_table(
                Table::create()
                .table(PartsByModel::Table)
                .if_not_exists()
                .col(uuid(PartsByModel::Id).primary_key())
                .col(uuid(PartsByModel::ProductId))
                .col(string(PartsByModel::PartNumber))
                .col(integer(PartsByModel::Quantity))
                .col(integer(PartsByModel::PartStatusId))
                .col(timestamp(CreatedAt))
                .col(timestamp(UpdatedAt))
                .col(timestamp_null(DeletedAt))
                .foreign_key(
                        ForeignKey::create()
                        .name("fk_part_by_model_part_type")
                        .from(PartsByModel::Table, PartsByModel::PartNumber)
                        .to(PartTypes::Table, PartTypes::PartNumber)
                        .on_update(ForeignKeyAction::Cascade)
                        .on_delete(ForeignKeyAction::Restrict),

                    )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_part_by_model_product")
                    .from(PartsByModel::Table, PartsByModel::ProductId)
                    .to(Products::Table, Products::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_part_installation_part_status")
                    .from(PartsByModel::Table, PartsByModel::PartStatusId)
                    .to(PartStatus::Table, PartStatus::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                )
                .to_owned()   
            )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        
        manager
            .drop_table(Table::drop().table(PartsByModel::Table).to_owned())
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
enum PartTypes {
    Table,
    PartNumber,
}

#[derive(DeriveIden)]
enum PartsByModel 
{
    Table,
    Id,
    PartNumber,
    ProductId,
    PartStatusId,
    Quantity,
}

#[derive(DeriveIden)]
enum Products
{
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartStatus
{
    Table,
    Id,
}

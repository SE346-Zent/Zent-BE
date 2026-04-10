use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // PartCatalog
        manager
            .create_table(
                Table::create()
                    .table(PartCatalog::Table)
                    .if_not_exists()
                    .col(uuid(PartCatalog::Id).primary_key())
                    .col(string(PartCatalog::PartNumber))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_catalog_part_types")
                            .from(PartCatalog::Table, PartCatalog::PartNumber)
                            .to(PartTypes::Table, PartTypes::PartNumber)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Parts
        manager
            .create_table(
                Table::create()
                    .table(Parts::Table)
                    .if_not_exists()
                    .col(uuid(Parts::Id).primary_key())
                    .col(uuid(Parts::CatalogId))
                    .col(uuid(Parts::ProductId))
                    .col(string(Parts::SerialNumber))
                    .col(uuid(Parts::PartStatusId))
                    .col(timestamp(Parts::MFD))
                    .col(timestamp(Parts::InstalledDate))
                    .col(timestamp_null(Parts::RemovedDate))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_part_catalog")
                            .from(Parts::Table, Parts::CatalogId)
                            .to(PartCatalog::Table, PartCatalog::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_part_status")
                            .from(Parts::Table, Parts::PartStatusId)
                            .to(PartStatus::Table, PartStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_product")
                            .from(Parts::Table, Parts::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Update PartsByModel to add missing references
        // SQLite only supports one alter operation per statement
        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .add_column(uuid_null(PartsByModel::CatalogId))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .add_column(integer_null(PartsByModel::ModelId))
                    .to_owned(),
            )
            .await?;

        // Note: SQLite does not support adding foreign key constraints to existing tables.
        // FK relationships for CatalogId -> PartCatalog and ModelId -> ProductModels
        // are enforced at the application/ORM level.

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop added columns (no FK to drop since SQLite doesn't support ALTER TABLE ADD FK)
        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .drop_column(PartsByModel::ModelId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .drop_column(PartsByModel::CatalogId)
                    .to_owned(),
            )
            .await?;

        manager.drop_table(Table::drop().table(Parts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartCatalog::Table).to_owned()).await?;

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
enum PartCatalog {
    Table,
    Id,
    PartNumber,
}

#[derive(DeriveIden)]
enum Parts {
    Table,
    Id,
    CatalogId,
    ProductId,
    SerialNumber,
    PartStatusId,
    MFD,
    InstalledDate,
    RemovedDate
}

#[derive(DeriveIden)]
enum PartTypes {
    Table,
    PartNumber,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartsByModel {
    Table,
    CatalogId,
    ModelId,
}

#[derive(DeriveIden)]
enum PartStatus {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}

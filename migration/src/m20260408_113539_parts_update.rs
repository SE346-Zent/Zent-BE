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
                    .col(string(Parts::SerialNumber))
                    .col(uuid(Parts::PartStatusId))
                    .col(timestamp(Parts::MFD))
                    .col(timestamp_null(Parts::AssemblyDate))
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
                    .to_owned(),
            )
            .await?;

        // Update PartsByModel to add missing references
        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .add_column(uuid_null(PartsByModel::CatalogId))
                    .add_column(integer_null(PartsByModel::ModelId))
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_parts_by_model_part_catalog")
                            .from_tbl(PartsByModel::Table)
                            .from_col(PartsByModel::CatalogId)
                            .to_tbl(PartCatalog::Table)
                            .to_col(PartCatalog::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_parts_by_model_product_models")
                            .from_tbl(PartsByModel::Table)
                            .from_col(PartsByModel::ModelId)
                            .to_tbl(ProductModels::Table)
                            .to_col(ProductModels::ModelCode)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .drop_foreign_key(Alias::new("fk_parts_by_model_product_models"))
                    .drop_foreign_key(Alias::new("fk_parts_by_model_part_catalog"))
                    .drop_column(PartsByModel::ModelId)
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
    SerialNumber,
    PartStatusId,
    MFD,
    AssemblyDate
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

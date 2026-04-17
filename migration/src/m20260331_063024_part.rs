use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        // PartCatalog (moved from parts_update, needed by PartsByModel.CatalogId)
        manager
            .create_table(
                Table::create()
                    .table(PartCatalog::Table)
                    .if_not_exists()
                    .col(uuid(PartCatalog::Id).primary_key())
                    .col(string(PartCatalog::PartNumber))
                    .col(string(PartCatalog::PartTypeId))
                    .col(integer(PartCatalog::PartMfgStatus))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_catalog_part_types")
                            .from(PartCatalog::Table, PartCatalog::PartTypeId)
                            .to(PartTypes::Table, PartTypes::PartTypeName)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_catalog_part_mfg_status")
                            .from(PartCatalog::Table, PartCatalog::PartMfgStatus)
                            .to(PartMfgStatuses::Table, PartMfgStatuses::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // PartConditions
        manager
            .create_table(
                Table::create()
                    .table(PartConditions::Table)
                    .if_not_exists()
                    .col(pk_auto(PartConditions::Id))
                    .col(string(PartConditions::Name))
                    .to_owned()
            )
            .await?;

        // PartsByModel with composite PK
        manager
            .create_table(
                Table::create()
                .table(PartsByModel::Table)
                .if_not_exists()
                .col(uuid(PartsByModel::PartCatalogId))
                .col(string(PartsByModel::ProductModelCode))
                .primary_key(
                    Index::create()
                        .col(PartsByModel::PartCatalogId)
                        .col(PartsByModel::ProductModelCode)
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_part_by_model_catalog")
                    .from(PartsByModel::Table, PartsByModel::PartCatalogId)
                    .to(PartCatalog::Table, PartCatalog::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_part_by_model_product_model")
                    .from(PartsByModel::Table, PartsByModel::ProductModelCode)
                    .to(ProductModels::Table, ProductModels::ModelCode)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::Restrict)
                )
                .to_owned()   
            )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse dependency order
        manager
            .drop_table(Table::drop().table(PartsByModel::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PartConditions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PartCatalog::Table).to_owned())
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
    PartTypeName,
}

#[derive(DeriveIden)]
enum PartsByModel 
{
    Table,
    PartCatalogId,
    ProductModelCode,
}

#[derive(DeriveIden)]
enum PartMfgStatuses
{
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartCatalog {
    Table,
    Id,
    PartNumber,
    PartMfgStatus,
    PartTypeId,
}

#[derive(DeriveIden)]
enum PartConditions {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}

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
                    .col(integer(PartCatalog::PartMfgStatusId))
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
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_catalog_part_mfg_status")
                            .from(PartCatalog::Table, PartCatalog::PartMfgStatusId)
                            .to(PartStatus::Table, PartStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // PartCondition (moved from parts_update, needed by PartsByModel.PartConditionId)
        manager
            .create_table(
                Table::create()
                    .table(PartCondition::Table)
                    .if_not_exists()
                    .col(pk_auto(PartCondition::Id))
                    .col(string(PartCondition::ConditionName))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned()
            )
            .await?;

        // PartsByModel with all columns and FK constraints
        manager
            .create_table(
                Table::create()
                .table(PartsByModel::Table)
                .if_not_exists()
                .col(pk_auto(PartsByModel::Id))
                .col(uuid(PartsByModel::PartCatalogId))
                .col(string(PartsByModel::ProductModelCode))
                .col(timestamp(CreatedAt))
                .col(timestamp(UpdatedAt))
                .col(timestamp_null(DeletedAt))
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
            .drop_table(Table::drop().table(PartCondition::Table).to_owned())
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
    PartNumber,
}

#[derive(DeriveIden)]
enum PartsByModel 
{
    Table,
    Id,
    PartCatalogId,
    ProductModelCode,
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

#[derive(DeriveIden)]
enum PartCatalog {
    Table,
    Id,
    PartNumber,
    PartMfgStatusId
}

#[derive(DeriveIden)]
enum PartCondition {
    Table,
    Id,
    ConditionName,
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}

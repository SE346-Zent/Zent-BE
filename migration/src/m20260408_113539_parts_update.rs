use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute(sea_orm_migration::sea_orm::Statement::from_string(manager.get_database_backend(), "PRAGMA foreign_keys = ON;".to_owned())).await?;

        // Parts table
        // PartCatalog and PartCondition are now created in the part migration
        manager
            .create_table(
                Table::create()
                    .table(Parts::Table)
                    .if_not_exists()
                    .col(uuid(Parts::Id).primary_key())
                    .col(uuid(Parts::PartCatalogId))
                    .col(uuid_null(Parts::ProductId))
                    .col(string(Parts::SerialNumber))
                    .col(integer(Parts::PartConditionId))
                    .col(timestamp(Parts::ManufacturedDate))
                    .col(timestamp_null(Parts::InstallationDate))
                    .col(timestamp_null(Parts::RemovalDate))
                    .col(timestamp_null(Parts::ScrappedDate))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_part_catalog")
                            .from(Parts::Table, Parts::PartCatalogId)
                            .to(PartCatalog::Table, PartCatalog::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_part_condition")
                            .from(Parts::Table, Parts::PartConditionId)
                            .to(PartConditions::Table, PartConditions::Id)
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

        manager
            .create_table(
                Table::create()
                    .table(PartChanges::Table)
                    .if_not_exists()
                    .col(uuid(PartChanges::PartId))
                    .col(uuid(PartChanges::WorkOrderClosingFormId))
                    .col(string(PartChanges::ChangeType))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .primary_key(
                        Index::create() 
                            .col(PartChanges::PartId)
                            .col(PartChanges::WorkOrderClosingFormId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_changes_wo")
                            .from(PartChanges::Table, PartChanges::WorkOrderClosingFormId)
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_changes_part")
                            .from(PartChanges::Table, PartChanges::PartId)
                            .to(Parts::Table, Parts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProductImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(ProductImageLinks::ImageId))
                    .col(uuid(ProductImageLinks::ProductId))
                    .primary_key(
                        Index::create()
                            .col(ProductImageLinks::ImageId)
                            .col(ProductImageLinks::ProductId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_image_links_image")
                            .from(ProductImageLinks::Table, ProductImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_product_image_links_product")
                            .from(ProductImageLinks::Table, ProductImageLinks::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProductModelImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(ProductModelImageLinks::ImageId))
                    .col(string(ProductModelImageLinks::ProductModelCode))
                    .primary_key(
                        Index::create()
                            .col(ProductModelImageLinks::ImageId)
                            .col(ProductModelImageLinks::ProductModelCode)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_model_image_links_image")
                            .from(ProductModelImageLinks::Table, ProductModelImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_model_image_links_model")
                            .from(ProductModelImageLinks::Table, ProductModelImageLinks::ProductModelCode)
                            .to(ProductModels::Table, ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(PartImageLinks::ImageId))
                    .col(uuid(PartImageLinks::PartId))
                    .primary_key(
                        Index::create()
                            .col(PartImageLinks::ImageId)
                            .col(PartImageLinks::PartId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_image_links_image")
                            .from(PartImageLinks::Table, PartImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_image_links_part")
                            .from(PartImageLinks::Table, PartImageLinks::PartId)
                            .to(Parts::Table, Parts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartCatalogImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(PartCatalogImageLinks::ImageId))
                    .col(uuid(PartCatalogImageLinks::PartCatalogId))
                    .primary_key(
                        Index::create()
                            .col(PartCatalogImageLinks::ImageId)
                            .col(PartCatalogImageLinks::PartCatalogId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_catalog_image_links_image")
                            .from(PartCatalogImageLinks::Table, PartCatalogImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_catalog_image_links_catalog")
                            .from(PartCatalogImageLinks::Table, PartCatalogImageLinks::PartCatalogId)
                            .to(PartCatalog::Table, PartCatalog::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PartCatalogImageLinks::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(PartImageLinks::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(ProductModelImageLinks::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(ProductImageLinks::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(PartChanges::Table).if_exists().to_owned()).await?;
        manager.drop_table(Table::drop().table(Parts::Table).if_exists().to_owned()).await?;

        Ok(())
    }
}

// Iden declarations for FK references to tables created in earlier migrations

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
}

#[derive(DeriveIden)]
enum Parts {
    Table,
    Id,
    PartCatalogId,
    ProductId,
    PartConditionId,
    SerialNumber,
    ManufacturedDate,
    InstallationDate,
    RemovalDate,
    ScrappedDate
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartConditions {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartChanges { 
    Table, 
    WorkOrderClosingFormId,
    PartId,
    ChangeType
}

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}

#[derive(DeriveIden)]
enum ProductImageLinks {
    Table,
    ImageId,
    ProductId,
}

#[derive(DeriveIden)]
enum ProductModelImageLinks {
    Table,
    ImageId,
    ProductModelCode,
}

#[derive(DeriveIden)]
enum PartImageLinks {
    Table,
    ImageId,
    PartId,
}

#[derive(DeriveIden)]
enum PartCatalogImageLinks {
    Table,
    ImageId,
    PartCatalogId,
}


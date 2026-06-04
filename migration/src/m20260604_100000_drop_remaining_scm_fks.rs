use sea_orm_migration::prelude::*;

/// Drop all remaining foreign keys that reference SCM-managed tables
/// (parts, part_catalog, products, product_models).
///
/// These tables are now owned by the SCM service (separate SQLite DB).
/// Zent-BE keeps local copies for read/reference but must not enforce
/// referential integrity via FK constraints against them.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── part_changes → parts ─────────────────────────────────────
        // This was the root cause of the completion form FK error.
        manager
            .alter_table(
                Table::alter()
                    .table(PartChanges::Table)
                    .drop_foreign_key(Alias::new("fk_part_changes_part"))
                    .to_owned(),
            )
            .await?;

        // ── parts → part_catalog ─────────────────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(Parts::Table)
                    .drop_foreign_key(Alias::new("fk_parts_part_catalog"))
                    .to_owned(),
            )
            .await?;

        // ── parts → products ─────────────────────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(Parts::Table)
                    .drop_foreign_key(Alias::new("fk_parts_product"))
                    .to_owned(),
            )
            .await?;

        // ── products → product_models ────────────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .drop_foreign_key(Alias::new("fk_products_model"))
                    .to_owned(),
            )
            .await?;

        // ── product_image_links → products ───────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(ProductImageLinks::Table)
                    .drop_foreign_key(Alias::new("fk_product_image_links_product"))
                    .to_owned(),
            )
            .await?;

        // ── product_model_image_links → product_models ───────────────
        manager
            .alter_table(
                Table::alter()
                    .table(ProductModelImageLinks::Table)
                    .drop_foreign_key(Alias::new("fk_model_image_links_model"))
                    .to_owned(),
            )
            .await?;

        // ── part_image_links → parts ─────────────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(PartImageLinks::Table)
                    .drop_foreign_key(Alias::new("fk_part_image_links_part"))
                    .to_owned(),
            )
            .await?;

        // ── part_catalog_image_links → part_catalog ──────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(PartCatalogImageLinks::Table)
                    .drop_foreign_key(Alias::new("fk_catalog_image_links_catalog"))
                    .to_owned(),
            )
            .await?;

        // ── parts_by_model → part_catalog ────────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .drop_foreign_key(Alias::new("fk_part_by_model_catalog"))
                    .to_owned(),
            )
            .await?;

        // ── parts_by_model → product_models ──────────────────────────
        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .drop_foreign_key(Alias::new("fk_part_by_model_product_model"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Restore all FK constraints (reverse order).

        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_part_by_model_product_model")
                            .from_tbl(PartsByModel::Table)
                            .from_col(PartsByModel::ProductModelCode)
                            .to_tbl(ProductModels::Table)
                            .to_col(ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PartsByModel::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_part_by_model_catalog")
                            .from_tbl(PartsByModel::Table)
                            .from_col(PartsByModel::PartCatalogId)
                            .to_tbl(PartCatalog::Table)
                            .to_col(PartCatalog::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PartCatalogImageLinks::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_catalog_image_links_catalog")
                            .from_tbl(PartCatalogImageLinks::Table)
                            .from_col(PartCatalogImageLinks::PartCatalogId)
                            .to_tbl(PartCatalog::Table)
                            .to_col(PartCatalog::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PartImageLinks::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_part_image_links_part")
                            .from_tbl(PartImageLinks::Table)
                            .from_col(PartImageLinks::PartId)
                            .to_tbl(Parts::Table)
                            .to_col(Parts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ProductModelImageLinks::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_model_image_links_model")
                            .from_tbl(ProductModelImageLinks::Table)
                            .from_col(ProductModelImageLinks::ProductModelCode)
                            .to_tbl(ProductModels::Table)
                            .to_col(ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ProductImageLinks::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_product_image_links_product")
                            .from_tbl(ProductImageLinks::Table)
                            .from_col(ProductImageLinks::ProductId)
                            .to_tbl(Products::Table)
                            .to_col(Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Products::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_products_model")
                            .from_tbl(Products::Table)
                            .from_col(Products::ProductModelCode)
                            .to_tbl(ProductModels::Table)
                            .to_col(ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Parts::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_parts_product")
                            .from_tbl(Parts::Table)
                            .from_col(Parts::ProductId)
                            .to_tbl(Products::Table)
                            .to_col(Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Parts::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_parts_part_catalog")
                            .from_tbl(Parts::Table)
                            .from_col(Parts::PartCatalogId)
                            .to_tbl(PartCatalog::Table)
                            .to_col(PartCatalog::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PartChanges::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_part_changes_part")
                            .from_tbl(PartChanges::Table)
                            .from_col(PartChanges::PartId)
                            .to_tbl(Parts::Table)
                            .to_col(Parts::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum PartChanges {
    Table,
    PartId,
}

#[derive(DeriveIden)]
enum Parts {
    Table,
    Id,
    PartCatalogId,
    ProductId,
}

#[derive(DeriveIden)]
enum PartCatalog {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
    ProductModelCode,
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}

#[derive(DeriveIden)]
enum ProductImageLinks {
    Table,
    ProductId,
}

#[derive(DeriveIden)]
enum ProductModelImageLinks {
    Table,
    ProductModelCode,
}

#[derive(DeriveIden)]
enum PartImageLinks {
    Table,
    PartId,
}

#[derive(DeriveIden)]
enum PartCatalogImageLinks {
    Table,
    PartCatalogId,
}

#[derive(DeriveIden)]
enum PartsByModel {
    Table,
    PartCatalogId,
    ProductModelCode,
}

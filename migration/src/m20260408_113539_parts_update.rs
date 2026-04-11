use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Parts table
        // PartCatalog and PartCondition are now created in the part migration
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Parts::Table).to_owned()).await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

#[derive(DeriveIden)]
struct DeletedAt;

// Iden declarations for FK references to tables created in earlier migrations

#[derive(DeriveIden)]
enum PartCatalog {
    Table,
    Id,
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
enum Products {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartStatus {
    Table,
    Id,
}

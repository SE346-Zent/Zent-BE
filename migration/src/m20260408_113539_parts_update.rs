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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PartChanges::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Parts::Table).to_owned()).await?;

        Ok(())
    }
}

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

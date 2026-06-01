use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WarrantyStatuses::Table)
                    .if_not_exists()
                    .col(pk_auto(WarrantyStatuses::Id))
                    .col(string(WarrantyStatuses::Name).unique_key())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Warranties::Table)
                    .add_column(integer_null(Warranties::WarrantyStatusId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_warranty_status")
                    .from(Warranties::Table, Warranties::WarrantyStatusId)
                    .to(WarrantyStatuses::Table, WarrantyStatuses::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_warranty_status")
                    .table(Warranties::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Warranties::Table)
                    .drop_column(Warranties::WarrantyStatusId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(WarrantyStatuses::Table).if_exists().to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WarrantyStatuses {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum Warranties {
    Table,
    WarrantyStatusId,
}
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the foreign key constraint to products table
        // since product_id references Zeus SCM products, not local products table
        manager
            .alter_table(
                Table::alter()
                    .table(RegisteredDevices::Table)
                    .drop_foreign_key(
                        ForeignKey::drop()
                            .name("fk_registered_devices_product")
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Re-add the foreign key constraint (for rollback)
        manager
            .alter_table(
                Table::alter()
                    .table(RegisteredDevices::Table)
                    .add_foreign_key(
                        ForeignKey::create()
                            .name("fk_registered_devices_product")
                            .from(RegisteredDevices::Table, RegisteredDevices::ProductId)
                            .to(Products::Table, Products::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RegisteredDevices {
    Table,
    ProductId,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

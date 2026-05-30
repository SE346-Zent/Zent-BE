use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RegisteredDevices::Table)
                    .if_not_exists()
                    .col(uuid(RegisteredDevices::Id).primary_key())
                    .col(uuid(RegisteredDevices::CustomerId))
                    .col(uuid(RegisteredDevices::ProductId))
                    .col(string(RegisteredDevices::Country).default("Vietnam"))
                    .col(string(RegisteredDevices::Province))
                    .col(string(RegisteredDevices::Ward))
                    .col(string(RegisteredDevices::Address))
                    .col(string(RegisteredDevices::FirstName))
                    .col(string(RegisteredDevices::LastName))
                    .col(string(RegisteredDevices::Email))
                    .col(string(RegisteredDevices::MobilePhone))
                    .col(boolean(RegisteredDevices::EmailConfirmationSent).default(false))
                    .col(timestamp(RegisteredDevices::CreatedAt))
                    .col(timestamp(RegisteredDevices::UpdatedAt))
                    .col(timestamp_null(RegisteredDevices::DeletedAt))
                    .index(
                        Index::create()
                            .name("idx_registered_devices_customer_product")
                            .col(RegisteredDevices::CustomerId)
                            .col(RegisteredDevices::ProductId)
                            .unique(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_registered_devices_customer")
                            .from(RegisteredDevices::Table, RegisteredDevices::CustomerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_registered_devices_product")
                            .from(RegisteredDevices::Table, RegisteredDevices::ProductId)
                            .to(Products::Table, Products::Id)
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
            .drop_table(Table::drop().table(RegisteredDevices::Table).if_exists().to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RegisteredDevices {
    Table,
    Id,
    CustomerId,
    ProductId,
    Country,
    Province,
    Ward,
    Address,
    FirstName,
    LastName,
    Email,
    MobilePhone,
    EmailConfirmationSent,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

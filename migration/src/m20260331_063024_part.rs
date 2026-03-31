use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .create_table(
                Table::create()
                    .table(PartTypes::Table)
                    .if_not_exists()
                    .col(uuid(PartTypes::PartNumber).primary_key())
                    .col(string(PartTypes::CommodityType))
                    .col(string(PartTypes::Description))
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .drop_table(Table::drop().table(Post::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PartTypes {
    Table,
    PartNumber,
    CommodityType,
    Description
}

#[derive(DeriveIden)]
enum PartTransaction
{
    Table,
    Id,
    PartNumber,
    EquipmentId,
    Quantity,
}

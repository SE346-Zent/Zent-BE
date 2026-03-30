use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EquipmentModels::Table)
                    .if_not_exists()
                    .col(pk_auto(EquipmentModels::Id))
                    .col(string(EquipmentModels::ModelName))
                    .col(string(EquipmentModels::ModelCode))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(EquipmentStatus::Table)
                    .if_not_exists()
                    .col(pk_auto(EquipmentStatus::Id))
                    .col(string(EquipmentStatus::Name))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartStatus::Table)
                    .if_not_exists()
                    .col(pk_auto(PartStatus::Id))
                    .col(string(PartStatus::Name))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PartTypes::Table)
                    .if_not_exists()
                    .col(pk_auto(PartTypes::Id))
                    .col(string(PartTypes::Name))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Equipments::Table)
                    .if_not_exists()
                    .col(uuid(Equipments::Id).primary_key())
                    .col(integer(Equipments::EquipmentStatusId))
                    .col(integer(Equipments::ModelId))
                    .col(uuid(Equipments::CustomerId))
                    .col(string(Equipments::EquipmentName))
                    .col(string_null(Equipments::SerialNumber))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_equipments_status")
                            .from(Equipments::Table, Equipments::EquipmentStatusId)
                            .to(EquipmentStatus::Table, EquipmentStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_equipments_model")
                            .from(Equipments::Table, Equipments::ModelId)
                            .to(EquipmentModels::Table, EquipmentModels::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Parts::Table)
                    .if_not_exists()
                    .col(uuid(Parts::Id).primary_key())
                    .col(uuid_null(Parts::EquipmentId))
                    .col(integer(Parts::PartStatusId))
                    .col(uuid(Parts::CustomerId))
                    .col(string(Parts::PartName))
                    .col(integer(Parts::Quantity))
                    .col(string_null(Parts::SerialNumber))
                    .col(timestamp_null(Parts::LastModifiedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_status")
                            .from(Parts::Table, Parts::PartStatusId)
                            .to(PartStatus::Table, PartStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_parts_equipment")
                            .from(Parts::Table, Parts::EquipmentId)
                            .to(Equipments::Table, Equipments::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Warranties::Table)
                    .if_not_exists()
                    .col(uuid(Warranties::Id).primary_key())
                    .col(uuid(Warranties::CustomerId))
                    .col(uuid(Warranties::EquipmentId))
                    .col(timestamp(Warranties::StartDate))
                    .col(timestamp_null(Warranties::EndDate))
                    .col(string(Warranties::WarrantyStatus))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_warranty_equipment")
                            .from(Warranties::Table, Warranties::EquipmentId)
                            .to(Equipments::Table, Equipments::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Images::Table)
                    .if_not_exists()
                    .col(uuid(Images::Id).primary_key())
                    .col(string(Images::ImageURL))
                    .col(uuid_null(Images::PartId))
                    .col(uuid_null(Images::EquipmentId))
                    .col(timestamp(Images::CapturedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_part")
                            .from(Images::Table, Images::PartId)
                            .to(Parts::Table, Parts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_equipment")
                            .from(Images::Table, Images::EquipmentId)
                            .to(Equipments::Table, Equipments::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Images::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Warranties::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Parts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Equipments::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartTypes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartStatus::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(EquipmentStatus::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(EquipmentModels::Table).to_owned()).await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

#[derive(DeriveIden)]
struct DeletedAt;

#[derive(DeriveIden)]
enum Parts
{
    Table,
    Id,
    EquipmentId,
    PartStatusId,
    CustomerId,
    PartName,
    Quantity,
    SerialNumber,
    LastModifiedAt,
}

#[derive(DeriveIden)]
enum Equipments
{
    Table,
    Id,
    EquipmentStatusId,
    ModelId,
    CustomerId,
    EquipmentName,
    SerialNumber,   
}

#[derive(DeriveIden)]
enum EquipmentModels
{
    Table,
    Id,
    ModelName,
    ModelCode
}

#[derive(DeriveIden)]
enum PartStatus
{
    Table,
    Id,
    Name    
}

#[derive(DeriveIden)]
enum EquipmentStatus
{
    Table,
    Id,
    Name    
}

#[derive(DeriveIden)]
enum PartTypes
{
    Table,
    Id,
    Name
}

#[derive(DeriveIden)]
enum Warranties
{
    Table,
    Id,
    CustomerId,
    EquipmentId,
    StartDate,
    EndDate,
    WarrantyStatus
}

#[derive(DeriveIden)]
enum Images
{
    Table,
    Id,
    ImageURL,
    PartId,
    EquipmentId,
    CapturedAt
}


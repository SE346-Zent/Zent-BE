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
                    .table(PartType::Table)
                    .if_not_exists()
                    .col(pk_auto(PartType::Id))
                    .col(string(PartType::Name))
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
                    .table(Warranty::Table)
                    .if_not_exists()
                    .col(uuid(Warranty::Id).primary_key())
                    .col(uuid(Warranty::CustomerId))
                    .col(uuid(Warranty::EquipmentId))
                    .col(timestamp(Warranty::StartDate))
                    .col(timestamp_null(Warranty::EndDate))
                    .col(string(Warranty::WarrantyStatus))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_warranty_equipment")
                            .from(Warranty::Table, Warranty::EquipmentId)
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
                    .table(Image::Table)
                    .if_not_exists()
                    .col(uuid(Image::Id).primary_key())
                    .col(string(Image::ImageURL))
                    .col(uuid_null(Image::PartId))
                    .col(uuid_null(Image::EquipmentId))
                    .col(timestamp(Image::CapturedAt))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_part")
                            .from(Image::Table, Image::PartId)
                            .to(Parts::Table, Parts::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_images_equipment")
                            .from(Image::Table, Image::EquipmentId)
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
        manager.drop_table(Table::drop().table(Image::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Warranty::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Parts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Equipments::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartType::Table).to_owned()).await?;
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
enum PartType
{
    Table,
    Id,
    Name
}

#[derive(DeriveIden)]
enum Warranty
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
enum Image
{
    Table,
    Id,
    ImageURL,
    PartId,
    EquipmentId,
    CapturedAt
}


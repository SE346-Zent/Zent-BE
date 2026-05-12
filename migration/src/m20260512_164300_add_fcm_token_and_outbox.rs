use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add fcm_token and installation_id to `users` table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column_if_not_exists(ColumnDef::new(Users::FcmToken).string().null())
                    .add_column_if_not_exists(ColumnDef::new(Users::InstallationId).string().null())
                    .to_owned(),
            )
            .await?;

        // 2. Create `outbox_records` table
        manager
            .create_table(
                Table::create()
                    .table(OutboxRecords::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OutboxRecords::OutboxId).uuid().not_null().primary_key())
                    .col(ColumnDef::new(OutboxRecords::UserId).uuid().not_null())
                    .col(ColumnDef::new(OutboxRecords::NotificationId).uuid().not_null())
                    .col(ColumnDef::new(OutboxRecords::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(OutboxRecords::Delivered).boolean().not_null().default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_outbox_records_user_id")
                            .from(OutboxRecords::Table, OutboxRecords::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 3. Create index for user_id to quickly find pending outbox records
        manager
            .create_index(
                Index::create()
                    .name("idx_outbox_records_user_id")
                    .table(OutboxRecords::Table)
                    .col(OutboxRecords::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Drop `outbox_records` table
        manager
            .drop_table(Table::drop().table(OutboxRecords::Table).to_owned())
            .await?;

        // 2. Drop columns from `users` table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::FcmToken)
                    .drop_column(Users::InstallationId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    FcmToken,
    InstallationId,
}

#[derive(DeriveIden)]
enum OutboxRecords {
    Table,
    OutboxId,
    UserId,
    NotificationId,
    CreatedAt,
    Delivered,
}

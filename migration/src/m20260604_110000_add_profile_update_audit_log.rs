use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProfileUpdateAuditLogs::Table)
                    .if_not_exists()
                    .col(uuid(ProfileUpdateAuditLogs::Id).primary_key())
                    .col(uuid(ProfileUpdateAuditLogs::UserId))
                    .col(integer(ProfileUpdateAuditLogs::RoleId))
                    .col(string(ProfileUpdateAuditLogs::ChangedBy))
                    .col(ColumnDef::new(ProfileUpdateAuditLogs::OldValues).text().not_null())
                    .col(ColumnDef::new(ProfileUpdateAuditLogs::NewValues).text().not_null())
                    .col(string_null(ProfileUpdateAuditLogs::IpAddress))
                    .col(timestamp(ProfileUpdateAuditLogs::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_profile_audit_user")
                            .from(ProfileUpdateAuditLogs::Table, ProfileUpdateAuditLogs::UserId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_profile_audit_user_id")
                            .col(ProfileUpdateAuditLogs::UserId),
                    )
                    .index(
                        Index::create()
                            .name("idx_profile_audit_created_at")
                            .col(ProfileUpdateAuditLogs::CreatedAt),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProfileUpdateAuditLogs::Table).if_exists().to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProfileUpdateAuditLogs {
    Table,
    Id,
    UserId,
    RoleId,
    ChangedBy,
    OldValues,
    NewValues,
    IpAddress,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

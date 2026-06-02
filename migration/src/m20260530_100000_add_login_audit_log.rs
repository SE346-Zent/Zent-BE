use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LoginAuditLogs::Table)
                    .if_not_exists()
                    .col(uuid(LoginAuditLogs::Id).primary_key())
                    .col(uuid(LoginAuditLogs::UserId))
                    .col(uuid(LoginAuditLogs::SessionId))
                    .col(string(LoginAuditLogs::DeviceName))
                    .col(string_null(LoginAuditLogs::Location))
                    .col(string_len(LoginAuditLogs::IPAddress, 45))
                    .col(timestamp(LoginAuditLogs::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_login_audit_logs_user")
                            .from(LoginAuditLogs::Table, LoginAuditLogs::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_login_audit_logs_user_created_at")
                            .col(LoginAuditLogs::UserId)
                            .col(LoginAuditLogs::CreatedAt),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LoginAuditLogs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LoginAuditLogs {
    Table,
    Id,
    UserId,
    SessionId,
    DeviceName,
    Location,
    IPAddress,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
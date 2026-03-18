use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(uuid(User::Id).primary_key())
                    .col(string(User::FullName))
                    .col(string_uniq(User::Email))
                    .col(string(User::PasswordHash))
                    .col(string(User::PhoneNumber))
                    .col(integer(User::AccountStatus))
                    .col(integer(User::RoleID))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_account_status")
                            .from(User::Table, User::AccountStatus)
                            .to(AccountStatus::Table, AccountStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_role_id")
                            .from(User::Table, User::RoleID)
                            .to(Role::Table, Role::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Role::Table)
                    .if_not_exists()
                    .col(pk_auto(Role::Id))
                    .col(string(Role::Name))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AccountStatus::Table)
                    .if_not_exists()
                    .col(pk_auto(AccountStatus::Id))
                    .col(string(AccountStatus::Name))
                    .to_owned(),
            )
            .await?;

        // TODO: GH Issue #15: trigger to invalidate sessions when user is deleted or essential fields are updated
        // TODO: Untracked: logging login attempts
        // TODO: Untracked: Security audit log
        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(uuid(Session::Id).primary_key())
                    .col(uuid(Session::UserID))
                    .col(string_len_uniq(Session::RefreshTokenHash, 64)) // SHA-256 hash
                    .col(string(Session::DeviceFingerprint))
                    .col(string_len(Session::IPAddress, 45))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(Session::ExpiresAt))
                    .col(timestamp_null(Session::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_user_id")
                            .from(Session::Table, Session::UserID)
                            .to(User::Table, User::Id)
                            // users are soft-deleted
                            // restrict is implemented to prevent orphaned rows
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .index(Index::create().name("idx_session_id").col(Session::Id))
                    .index(
                        Index::create()
                            .name("idx_session_expires_at")
                            .col(Session::ExpiresAt),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Role::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AccountStatus::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

#[derive(DeriveIden)]
struct DeletedAt;

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    FullName,
    Email,
    PasswordHash,
    PhoneNumber,
    AccountStatus,
    RoleID,
}

#[derive(DeriveIden)]
enum Role {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum AccountStatus {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum Session {
    Table,
    Id,
    RefreshTokenHash,
    UserID,
    DeviceFingerprint,
    IPAddress,
    ExpiresAt,
    RevokedAt,
}

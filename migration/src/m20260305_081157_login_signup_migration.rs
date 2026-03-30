use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(uuid(Users::Id).primary_key())
                    .col(string(Users::FullName))
                    .col(string_uniq(Users::Email))
                    .col(string(Users::PasswordHash))
                    .col(string(Users::PhoneNumber))
                    .col(integer(Users::AccountStatus))
                    .col(integer(Users::RoleID))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(UpdatedAt))
                    .col(timestamp_null(DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_account_status")
                            .from(Users::Table, Users::AccountStatus)
                            .to(AccountStatus::Table, AccountStatus::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_role_id")
                            .from(Users::Table, Users::RoleID)
                            .to(Roles::Table, Roles::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Roles::Table)
                    .if_not_exists()
                    .col(pk_auto(Roles::Id))
                    .col(string(Roles::Name))
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
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(uuid(Sessions::Id).primary_key())
                    .col(uuid(Sessions::UserID))
                    .col(string_len_uniq(Sessions::RefreshTokenHash, 64)) // SHA-256 hash
                    .col(string(Sessions::DeviceFingerprint))
                    .col(string_len(Sessions::IPAddress, 45))
                    .col(timestamp(CreatedAt))
                    .col(timestamp(Sessions::ExpiresAt))
                    .col(timestamp_null(Sessions::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_user_id")
                            .from(Sessions::Table, Sessions::UserID)
                            .to(Users::Table, Users::Id)
                            // users are soft-deleted
                            // restrict is implemented to prevent orphaned rows
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_session_expires_at")
                    .table(Sessions::Table)
                    .col(Sessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Roles::Table).to_owned())
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
enum Users {
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
enum Roles {
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
enum Sessions {
    Table,
    Id,
    RefreshTokenHash,
    UserID,
    DeviceFingerprint,
    IPAddress,
    ExpiresAt,
    RevokedAt,
}

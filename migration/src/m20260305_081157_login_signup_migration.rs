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
                    .col(string(User::Email))
                    .col(string(User::PasswordHash))
                    .col(string(User::PhoneNumber))
                    .col(string(User::AccountStatus))
                    .col(string(User::RoleID))
                    .col(timestamp_null(CreatedAt))
                    .col(timestamp_null(UpdatedAt))
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

        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .if_not_exists()
                    .col(uuid(Session::Id).primary_key())
                    .col(uuid(Session::UserID))
                    .col(string(Session::AccessToken))
                    .col(string(Session::DeviceFingerprint))
                    .col(string(Session::IPAddress))
                    .col(timestamp_null(CreatedAt))
                    .col(timestamp_null(Session::ExpiresAt))
                    .col(timestamp_null(Session::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_session_user_id")
                            .from(Session::Table, Session::UserID)
                            .to(User::Table, User::Id)
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
            .drop_table(Table::drop().table(User::Table).to_owned())
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
    Id, // also refresh token
    UserID,
    AccessToken,
    DeviceFingerprint,
    IPAddress,
    ExpiresAt,
    RevokedAt,
}

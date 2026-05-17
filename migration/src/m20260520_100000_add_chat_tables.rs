use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. chat_rooms table
        manager
            .create_table(
                Table::create()
                    .table(ChatRooms::Table)
                    .if_not_exists()
                    .col(uuid(ChatRooms::Id).primary_key())
                    .col(string(ChatRooms::RoomName))
                    .col(uuid(ChatRooms::CreatedBy))
                    .col(timestamp(ChatRooms::CreatedAt).default(SimpleExpr::Keyword(Keyword::CurrentTimestamp)))
                    .col(timestamp_null(ChatRooms::UpdatedAt))
                    .col(timestamp_null(ChatRooms::DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chat_room_created_by")
                            .from(ChatRooms::Table, ChatRooms::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. chat_room_members table
        manager
            .create_table(
                Table::create()
                    .table(ChatRoomMembers::Table)
                    .if_not_exists()
                    .col(uuid(ChatRoomMembers::RoomId))
                    .col(uuid(ChatRoomMembers::UserId))
                    .col(timestamp(ChatRoomMembers::CreatedAt).default(SimpleExpr::Keyword(Keyword::CurrentTimestamp)))
                    .col(timestamp_null(ChatRoomMembers::UpdatedAt))
                    .col(timestamp_null(ChatRoomMembers::DeletedAt))
                    .primary_key(
                        Index::create()
                            .col(ChatRoomMembers::RoomId)
                            .col(ChatRoomMembers::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chat_member_room")
                            .from(ChatRoomMembers::Table, ChatRoomMembers::RoomId)
                            .to(ChatRooms::Table, ChatRooms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_chat_member_user")
                            .from(ChatRoomMembers::Table, ChatRoomMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChatRoomMembers::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ChatRooms::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    Id,
    RoomName,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum ChatRoomMembers {
    Table,
    RoomId,
    UserId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add avatar_url to users
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(string_null(Users::AvatarUrl))
                    .to_owned(),
            )
            .await?;

        // 2. Create chat_room_image_links table
        manager
            .create_table(
                Table::create()
                    .table(ChatRoomImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(ChatRoomImageLinks::ImageId))
                    .col(uuid(ChatRoomImageLinks::RoomId))
                    .col(timestamp(ChatRoomImageLinks::CreatedAt).default(SimpleExpr::Keyword(Keyword::CurrentTimestamp)))
                    .primary_key(
                        Index::create()
                            .col(ChatRoomImageLinks::ImageId)
                            .col(ChatRoomImageLinks::RoomId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cril_image")
                            .from(ChatRoomImageLinks::Table, ChatRoomImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cril_room")
                            .from(ChatRoomImageLinks::Table, ChatRoomImageLinks::RoomId)
                            .to(ChatRooms::Table, ChatRooms::Id)
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
            .drop_table(Table::drop().table(ChatRoomImageLinks::Table).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::AvatarUrl)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    AvatarUrl,
}

#[derive(DeriveIden)]
enum ChatRoomImageLinks {
    Table,
    ImageId,
    RoomId,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    Id,
}

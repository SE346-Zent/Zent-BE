use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add chat_room_id column to work_orders (nullable FK to chat_rooms)
        // This enforces the N:1 relationship: many work orders → one chat room.
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .add_column(
                        ColumnDef::new(WorkOrders::ChatRoomId)
                            .uuid()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_work_order_chat_room")
                    .from(WorkOrders::Table, WorkOrders::ChatRoomId)
                    .to(ChatRooms::Table, ChatRooms::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_work_order_chat_room")
                    .table(WorkOrders::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::ChatRoomId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    ChatRoomId,
}

#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    Id,
}

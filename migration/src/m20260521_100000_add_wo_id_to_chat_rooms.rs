use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ChatRooms::Table)
                    .add_column(uuid_null(ChatRooms::WorkOrderId))
                    .to_owned(),
            )
            .await?;

        // FK to work_orders (optional, delete set null on WO delete)
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_chat_room_work_order")
                    .from(ChatRooms::Table, ChatRooms::WorkOrderId)
                    .to(WorkOrders::Table, WorkOrders::Id)
                    .on_update(ForeignKeyAction::Cascade)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_chat_room_work_order")
                    .table(ChatRooms::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(ChatRooms::Table)
                    .drop_column(ChatRooms::WorkOrderId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum ChatRooms {
    Table,
    WorkOrderId,
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    Id,
}

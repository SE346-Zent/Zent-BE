use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Index on work_order_escalations.work_order_id for efficient lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_wo_escalations_work_order_id")
                    .table(WorkOrderEscalations::Table)
                    .col(WorkOrderEscalations::WorkOrderId)
                    .to_owned(),
            )
            .await?;

        // Index on chat_room_members.user_id for efficient room-listing queries
        manager
            .create_index(
                Index::create()
                    .name("idx_chat_room_members_user_id")
                    .table(ChatRoomMembers::Table)
                    .col(ChatRoomMembers::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_wo_escalations_work_order_id")
                    .table(WorkOrderEscalations::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_chat_room_members_user_id")
                    .table(ChatRoomMembers::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum WorkOrderEscalations {
    Table,
    WorkOrderId,
}

#[derive(DeriveIden)]
enum ChatRoomMembers {
    Table,
    UserId,
}

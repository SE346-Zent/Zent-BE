use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite supports only one ALTER per statement — split into individual calls.
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .add_column(string_null(WorkOrderRejectForms::Reason))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .add_column(string_null(WorkOrderRejectForms::Explanation))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .add_column(timestamp_null(CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .add_column(timestamp_null(UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkOrderRejectFormImageLinks::Table)
                    .if_not_exists()
                    .col(uuid(WorkOrderRejectFormImageLinks::ImageId))
                    .col(uuid(WorkOrderRejectFormImageLinks::WorkOrderRejectFormId))
                    .primary_key(
                        Index::create()
                            .col(WorkOrderRejectFormImageLinks::ImageId)
                            .col(WorkOrderRejectFormImageLinks::WorkOrderRejectFormId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rj_image_links_image")
                            .from(WorkOrderRejectFormImageLinks::Table, WorkOrderRejectFormImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rj_image_links_rj")
                            .from(WorkOrderRejectFormImageLinks::Table, WorkOrderRejectFormImageLinks::WorkOrderRejectFormId)
                            .to(WorkOrderRejectForms::Table, WorkOrderRejectForms::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(WorkOrderRejectFormImageLinks::Table)
                    .to_owned(),
            )
            .await?;

        // SQLite: one drop per ALTER
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .drop_column(WorkOrderRejectForms::Reason)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .drop_column(WorkOrderRejectForms::Explanation)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .drop_column(CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderRejectForms::Table)
                    .drop_column(UpdatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrderRejectForms {
    Table,
    Id,
    Reason,
    Explanation,
}

#[derive(DeriveIden)]
enum WorkOrderRejectFormImageLinks {
    Table,
    ImageId,
    WorkOrderRejectFormId,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
}

#[derive(DeriveIden)]
struct CreatedAt;

#[derive(DeriveIden)]
struct UpdatedAt;

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Drop closing_form_image_links table
        manager
            .drop_table(Table::drop().table(ClosingFormImageLinks::Table).to_owned())
            .await?;

        // 2. Rename work_order_closing_image_links to work_order_image_links
        manager
            .rename_table(
                Table::rename()
                    .table(WorkOrderClosingImageLinks::Table, WorkOrderImageLinks::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Rename work_order_image_links back to work_order_closing_image_links
        manager
            .rename_table(
                Table::rename()
                    .table(WorkOrderImageLinks::Table, WorkOrderClosingImageLinks::Table)
                    .to_owned(),
            )
            .await?;

        // 2. Re-create closing_form_image_links table
        manager
            .create_table(
                Table::create()
                    .table(ClosingFormImageLinks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ClosingFormImageLinks::ImageId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ClosingFormImageLinks::WorkOrderClosingFormId)
                            .uuid()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk-closing_form_image_links")
                            .col(ClosingFormImageLinks::ImageId)
                            .col(ClosingFormImageLinks::WorkOrderClosingFormId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-closing_form_image_links-image_id")
                            .from(ClosingFormImageLinks::Table, ClosingFormImageLinks::ImageId)
                            .to(Images::Table, Images::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-closing_form_image_links-form_id")
                            .from(
                                ClosingFormImageLinks::Table,
                                ClosingFormImageLinks::WorkOrderClosingFormId,
                            )
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ClosingFormImageLinks {
    Table,
    ImageId,
    WorkOrderClosingFormId,
}

#[derive(DeriveIden)]
enum WorkOrderClosingImageLinks {
    Table,
}

#[derive(DeriveIden)]
enum WorkOrderImageLinks {
    Table,
}

#[derive(DeriveIden)]
enum Images {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    Id,
}

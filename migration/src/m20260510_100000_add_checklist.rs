use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ===== 1. Create checklist_items table =====
        manager
            .create_table(
                Table::create()
                    .table(ChecklistItems::Table)
                    .if_not_exists()
                    .col(pk_auto(ChecklistItems::Id))
                    .col(string(ChecklistItems::Name))
                    .col(timestamp(ChecklistItems::CreatedAt))
                    .col(timestamp(ChecklistItems::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        // ===== 2. Create junction table: work_order_closing_form_checklist_results =====
        manager
            .create_table(
                Table::create()
                    .table(ClosingFormChecklistResults::Table)
                    .if_not_exists()
                    .col(uuid(ClosingFormChecklistResults::ClosingFormId))
                    .col(integer(ClosingFormChecklistResults::ChecklistItemId))
                    .col(boolean(ClosingFormChecklistResults::Result))
                    .col(string_null(ClosingFormChecklistResults::Notes))
                    .col(timestamp(ClosingFormChecklistResults::CreatedAt))
                    .primary_key(
                        Index::create()
                            .col(ClosingFormChecklistResults::ClosingFormId)
                            .col(ClosingFormChecklistResults::ChecklistItemId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_checklist_results_closing_form")
                            .from(
                                ClosingFormChecklistResults::Table,
                                ClosingFormChecklistResults::ClosingFormId,
                            )
                            .to(WorkOrderClosingForms::Table, WorkOrderClosingForms::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_checklist_results_item")
                            .from(
                                ClosingFormChecklistResults::Table,
                                ClosingFormChecklistResults::ChecklistItemId,
                            )
                            .to(ChecklistItems::Table, ChecklistItems::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
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
                    .table(ClosingFormChecklistResults::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(
                Table::drop()
                    .table(ChecklistItems::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ChecklistItems {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ClosingFormChecklistResults {
    Table,
    ClosingFormId,
    ChecklistItemId,
    Result,
    Notes,
    CreatedAt,
}

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    Id,
}

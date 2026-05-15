use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add `status` column to new_part_forms (pending / approved / denied)
        if !manager.has_column("new_part_forms", "status").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .add_column(
                            ColumnDef::new(NewPartForms::Status)
                                .string()
                                .not_null()
                                .default("pending"),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 2. Add `denial_reason` column to new_part_forms (nullable)
        if !manager.has_column("new_part_forms", "denial_reason").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .add_column(ColumnDef::new(NewPartForms::DenialReason).text().null())
                        .to_owned(),
                )
                .await?;
        }

        // 3. Create `part_audit_log` table
        manager
            .create_table(
                Table::create()
                    .table(PartAuditLog::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PartAuditLog::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(PartAuditLog::NewPartFormId).uuid().not_null())
                    .col(ColumnDef::new(PartAuditLog::Action).string().not_null())
                    .col(ColumnDef::new(PartAuditLog::AdminId).uuid().not_null())
                    .col(ColumnDef::new(PartAuditLog::Reason).text().null())
                    .col(ColumnDef::new(PartAuditLog::CreatedAt).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_audit_log_new_part_form")
                            .from(PartAuditLog::Table, PartAuditLog::NewPartFormId)
                            .to(NewPartForms::Table, NewPartForms::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_part_audit_log_admin")
                            .from(PartAuditLog::Table, PartAuditLog::AdminId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Index on new_part_form_id for quick audit lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_part_audit_log_form_id")
                    .table(PartAuditLog::Table)
                    .col(PartAuditLog::NewPartFormId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PartAuditLog::Table).to_owned())
            .await?;

        if manager.has_column("new_part_forms", "denial_reason").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .drop_column(NewPartForms::DenialReason)
                        .to_owned(),
                )
                .await?;
        }

        if manager.has_column("new_part_forms", "status").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .drop_column(NewPartForms::Status)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    Id,
    Status,
    DenialReason,
}

#[derive(DeriveIden)]
enum PartAuditLog {
    Table,
    Id,
    NewPartFormId,
    Action,
    AdminId,
    Reason,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

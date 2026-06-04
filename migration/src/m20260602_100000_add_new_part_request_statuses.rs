use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Create new_part_request_statuses lookup table
        manager
            .create_table(
                Table::create()
                    .table(NewPartRequestStatuses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NewPartRequestStatuses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(NewPartRequestStatuses::Name)
                            .string_len(50)
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Seed statuses
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            INSERT INTO new_part_request_statuses (id, name) VALUES
                (1, 'pending'),
                (2, 'approved'),
                (3, 'rejected')
            ON DUPLICATE KEY UPDATE name = VALUES(name)
            "#,
        )
        .await?;

        // 3. Add new_part_request_status_id column to new_part_forms
        if !manager
            .has_column("new_part_forms", "new_part_request_status_id")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .add_column(
                            ColumnDef::new(NewPartForms::NewPartRequestStatusId)
                                .integer()
                                .not_null()
                                .default(1),
                        )
                        .to_owned(),
                )
                .await?;

            // Add foreign key
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .add_foreign_key(
                            TableForeignKey::new()
                                .name("fk_new_part_forms_status")
                                .from_tbl(NewPartForms::Table)
                                .from_col(NewPartForms::NewPartRequestStatusId)
                                .to_tbl(NewPartRequestStatuses::Table)
                                .to_col(NewPartRequestStatuses::Id)
                                .on_delete(ForeignKeyAction::Restrict)
                                .on_update(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 4. Backfill status_id from the existing string status column
        db.execute_unprepared(
            r#"
            UPDATE new_part_forms
            SET new_part_request_status_id = CASE
                WHEN LOWER(status) = 'pending' THEN 1
                WHEN LOWER(status) = 'approved' THEN 2
                WHEN LOWER(status) IN ('rejected', 'denied') THEN 3
                ELSE 1
            END
            "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager
            .has_column("new_part_forms", "new_part_request_status_id")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .drop_foreign_key(Alias::new("fk_new_part_forms_status"))
                        .to_owned(),
                )
                .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(NewPartForms::Table)
                        .drop_column(NewPartForms::NewPartRequestStatusId)
                        .to_owned(),
                )
                .await?;
        }

        manager
            .drop_table(Table::drop().table(NewPartRequestStatuses::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum NewPartRequestStatuses {
    Table,
    Id,
    Name,
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    NewPartRequestStatusId,
}

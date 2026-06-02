use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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

        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            UPDATE new_part_forms
            SET status = CASE
                WHEN EXISTS (
                    SELECT 1
                                        FROM part_audit_log pal
                    WHERE pal.new_part_form_id = new_part_forms.id
                      AND LOWER(pal.action) = 'approved'
                ) THEN 'approved'
                WHEN EXISTS (
                    SELECT 1
                                        FROM part_audit_log pal
                    WHERE pal.new_part_form_id = new_part_forms.id
                      AND LOWER(pal.action) IN ('denied', 'rejected')
                ) THEN 'rejected'
                ELSE COALESCE(status, 'pending')
            END
            "#,
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "UPDATE new_part_forms SET status = CASE WHEN status = 'rejected' THEN 'denied' ELSE status END",
        ).await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    Status,
}
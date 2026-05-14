use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add category_id column (if not already present — previous failed runs may have added it)
        if !manager.has_column("outbox_records", "category_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .add_column(ColumnDef::new(OutboxRecords::CategoryId).integer().not_null().default(0))
                        .to_owned(),
                )
                .await?;
        }

        // Add title column (if not already present)
        if !manager.has_column("outbox_records", "title").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .add_column(ColumnDef::new(OutboxRecords::Title).string().not_null().default(""))
                        .to_owned(),
                )
                .await?;
        }

        // Add body column (if not already present)
        if !manager.has_column("outbox_records", "body").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .add_column(ColumnDef::new(OutboxRecords::Body).text().not_null())
                        .to_owned(),
                )
                .await?;
        }

        // Add data column (if not already present)
        // JSON stored as TEXT for MySQL compatibility.
        // Note: MySQL does not allow default values on TEXT columns; the app always sets a value.
        if !manager.has_column("outbox_records", "data").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .add_column(ColumnDef::new(OutboxRecords::Data).text().not_null())
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Only drop columns that still exist
        if manager.has_column("outbox_records", "data").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .drop_column(OutboxRecords::Data)
                        .to_owned(),
                )
                .await?;
        }

        if manager.has_column("outbox_records", "body").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .drop_column(OutboxRecords::Body)
                        .to_owned(),
                )
                .await?;
        }

        if manager.has_column("outbox_records", "title").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .drop_column(OutboxRecords::Title)
                        .to_owned(),
                )
                .await?;
        }

        if manager.has_column("outbox_records", "category_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(OutboxRecords::Table)
                        .drop_column(OutboxRecords::CategoryId)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum OutboxRecords {
    Table,
    CategoryId,
    Title,
    Body,
    Data,
}

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("images", "internet_time").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Images::Table)
                        .add_column(
                            ColumnDef::new(Images::InternetTime)
                                .big_integer()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_column("images", "internet_time").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Images::Table)
                        .drop_column(Images::InternetTime)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Images {
    Table,
    InternetTime,
}

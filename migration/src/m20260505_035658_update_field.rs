use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::State)
                    .add_column(ColumnDef::new(Users::Province).string().null())
                    .to_owned()
            )
            .await?;
        
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::State)
                    .add_column(ColumnDef::new(WorkOrders::Province).string().null())
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Province)
                    .add_column(ColumnDef::new(Users::State).string().null())
                    .to_owned()
            )
            .await?;
        
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_column(WorkOrders::Province)
                    .add_column(ColumnDef::new(WorkOrders::State).string().null())
                    .to_owned()
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    State,
    Province,
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    State,
    Province,
}

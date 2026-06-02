use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .drop_foreign_key(Alias::new("fk_work_order_product"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingForms::Table)
                    .drop_foreign_key(Alias::new("fk_wo_closing_forms_product"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Warranties::Table)
                    .drop_foreign_key(Alias::new("fk_warranty_product"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .drop_foreign_key(Alias::new("fk_new_part_form_part_type"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .drop_foreign_key(Alias::new("fk_new_part_form_product_model"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrders::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_work_order_product")
                            .from_tbl(WorkOrders::Table)
                            .from_col(WorkOrders::ProductId)
                            .to_tbl(Products::Table)
                            .to_col(Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkOrderClosingForms::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_wo_closing_forms_product")
                            .from_tbl(WorkOrderClosingForms::Table)
                            .from_col(WorkOrderClosingForms::ProductId)
                            .to_tbl(Products::Table)
                            .to_col(Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Warranties::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_warranty_product")
                            .from_tbl(Warranties::Table)
                            .from_col(Warranties::ProductId)
                            .to_tbl(Products::Table)
                            .to_col(Products::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_new_part_form_part_type")
                            .from_tbl(NewPartForms::Table)
                            .from_col(NewPartForms::PartTypesId)
                            .to_tbl(PartTypes::Table)
                            .to_col(PartTypes::Id)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(NewPartForms::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_new_part_form_product_model")
                            .from_tbl(NewPartForms::Table)
                            .from_col(NewPartForms::ModelCode)
                            .to_tbl(ProductModels::Table)
                            .to_col(ProductModels::ModelCode)
                            .on_update(ForeignKeyAction::Cascade)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorkOrders {
    Table,
    ProductId,
}

#[derive(DeriveIden)]
enum WorkOrderClosingForms {
    Table,
    ProductId,
}

#[derive(DeriveIden)]
enum Warranties {
    Table,
    ProductId,
}

#[derive(DeriveIden)]
enum NewPartForms {
    Table,
    PartTypesId,
    ModelCode,
}

#[derive(DeriveIden)]
enum Products {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum PartTypes {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ProductModels {
    Table,
    ModelCode,
}

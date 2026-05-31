//! `SeaORM` Entity for cancel_reasons table.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "cancel_reasons")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub cancelled_by: Uuid,
    pub reason: String,
    pub additional_comments: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::work_orders::Entity",
        from = "Column::WorkOrderId",
        to = "super::work_orders::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    WorkOrders,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CancelledBy",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Users,
}

impl Related<super::work_orders::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrders.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

//! `SeaORM` Entity for work_order_escalations audit table

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "work_order_escalations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_order_id: Uuid,
    /// 1 = 110%, 2 = 125%, 3 = 150%
    pub escalation_level: i32,
    pub elapsed_minutes: i64,
    pub baseline_minutes: i64,
    pub notified_sa_count: i32,
    pub notified_admin_count: i32,
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
}

impl Related<super::work_orders::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

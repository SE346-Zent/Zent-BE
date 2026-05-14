//! SeaORM Entity for `outbox_records` table.
//! Used for the Outbox Pattern to reliably dispatch FCM push notifications.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "outbox_records")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub outbox_id: Uuid,
    pub user_id: Uuid,
    /// References the MongoDB notification document ID (stored as UUID string).
    pub notification_id: Uuid,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    /// JSON payload stored as TEXT.
    pub data: String,
    pub created_at: DateTimeUtc,
    pub delivered: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

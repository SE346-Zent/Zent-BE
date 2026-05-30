//! SeaORM Entity for `new_part_audit_log` table.
//! Audit trail for new part approval/denial actions by admins.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "new_part_audit_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub new_part_form_id: Uuid,
    /// "approved" or "denied"
    pub action: String,
    pub admin_id: Uuid,
    /// Reason for denial (null when approved)
    pub reason: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::new_part_forms::Entity",
        from = "Column::NewPartFormId",
        to = "super::new_part_forms::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    NewPartForms,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::AdminId",
        to = "super::users::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Users,
}

impl Related<super::new_part_forms::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NewPartForms.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
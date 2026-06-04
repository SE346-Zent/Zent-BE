use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};

/// Audit log for staff profile updates (Technician, Admin, SuperAdmin).
/// Stores the previous and new values as JSON text for each update action.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "profile_update_audit_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// The user whose profile was updated.
    pub user_id: Uuid,
    /// Role of the user at the time of the update (for quick filtering).
    pub role_id: i32,
    /// Who performed the update (user ID as string — could be self or an admin).
    pub changed_by: String,
    /// JSON object of field values before the update.
    pub old_values: String,
    /// JSON object of field values after the update.
    pub new_values: String,
    /// IP address of the request.
    pub ip_address: Option<String>,
    pub created_at: DateTimeUtc,
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

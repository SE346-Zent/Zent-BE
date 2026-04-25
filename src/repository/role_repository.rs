use sea_orm::*;

use crate::entities::roles;

/// Find a role by its primary key.
pub async fn find_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<roles::Model>, DbErr> {
    roles::Entity::find_by_id(id).one(db).await
}

/// Find a role by its name.
pub async fn find_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<roles::Model>, DbErr> {
    roles::Entity::find()
        .filter(roles::Column::Name.eq(name))
        .one(db)
        .await
}

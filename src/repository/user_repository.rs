use sea_orm::*;
use uuid::Uuid;

use crate::entities::users;

/// Find a user by their email address.
pub async fn find_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<users::Model>, DbErr> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
}

/// Find a user by their primary key (UUID).
pub async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<users::Model>, DbErr> {
    users::Entity::find_by_id(id).one(db).await
}

/// Insert a new user record.
pub async fn create(
    db: &DatabaseConnection,
    model: users::ActiveModel,
) -> Result<users::Model, DbErr> {
    model.insert(db).await
}

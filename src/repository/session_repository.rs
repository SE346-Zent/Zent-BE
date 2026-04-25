use sea_orm::*;
use sea_orm::prelude::Expr;
use uuid::Uuid;

use crate::entities::sessions;

/// Insert a new session record.
pub async fn create(
    db: &DatabaseConnection,
    model: sessions::ActiveModel,
) -> Result<sessions::Model, DbErr> {
    model.insert(db).await
}

/// Find a session by its refresh token hash.
pub async fn find_by_hash(
    db: &DatabaseConnection,
    hash: &str,
) -> Result<Option<sessions::Model>, DbErr> {
    sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(hash))
        .one(db)
        .await
}

/// Update an existing session record.
pub async fn update(
    db: &DatabaseConnection,
    model: sessions::ActiveModel,
) -> Result<sessions::Model, DbErr> {
    model.update(db).await
}

/// Delete (revoke) a session by its ID.
pub async fn revoke(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<UpdateResult, DbErr> {
    sessions::Entity::update_many()
        .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
        .filter(sessions::Column::Id.eq(id))
        .exec(db)
        .await
}

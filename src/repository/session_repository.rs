use sea_orm::*;

use crate::entities::sessions;

/// Insert a new session record.
pub async fn create(
    db: &DatabaseConnection,
    model: sessions::ActiveModel,
) -> Result<sessions::Model, DbErr> {
    model.insert(db).await
}

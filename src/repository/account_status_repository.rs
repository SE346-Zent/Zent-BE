use sea_orm::*;

use crate::entities::account_status;

/// Find an account status by its name.
pub async fn find_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<account_status::Model>, DbErr> {
    account_status::Entity::find()
        .filter(account_status::Column::Name.eq(name))
        .one(db)
        .await
}

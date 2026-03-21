use sea_orm::*;
use uuid::Uuid;

use crate::{
    entities::{role, user},
    model::{
        responses::account::profile_response::{
            ProfileListQuery, ProfileListResponse, ProfileResponse, ProfileResponseData,
        },
        responses::error::AppError,
    },
};

pub async fn get_profile_service(
    db: DatabaseConnection,
    user_id: Uuid,
) -> Result<ProfileResponse, AppError> {
    let user_model = user::Entity::find_by_id(user_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("User not found".to_string()))?;

    let data = ProfileResponseData {
        full_name: user_model.full_name,
        email: user_model.email,
        phone_number: user_model.phone_number,
        role_id: user_model.role_id,
        account_status: user_model.account_status,
    };

    Ok(ProfileResponse::success(data))
}

pub async fn get_profiles_service(
    db: DatabaseConnection,
    query: ProfileListQuery,
) -> Result<ProfileListResponse, AppError> {
    // Lookup role id by name
    let role_model = role::Entity::find()
        .filter(role::Column::Name.eq(&query.role))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("Role not found".to_string()))?;

    // Count total items
    let user_query = user::Entity::find().filter(user::Column::RoleId.eq(role_model.id));
    let total_items = user_query
        .clone()
        .count(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    // Paginate
    let paginator = user_query.paginate(&db, query.pagination.per_page);
    
    // Page is 1-indexed in PaginationQuery, but fetch_page is 0-indexed in sea_orm.
    let page_index = query.pagination.page.saturating_sub(1);

    let users = paginator
        .fetch_page(page_index)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = users
        .into_iter()
        .map(|user_model| ProfileResponseData {
            full_name: user_model.full_name,
            email: user_model.email,
            phone_number: user_model.phone_number,
            role_id: user_model.role_id,
            account_status: user_model.account_status,
        })
        .collect::<Vec<_>>();

    Ok(ProfileListResponse::success(
        &query.role,
        data,
        &query.pagination,
        total_items,
    ))
}

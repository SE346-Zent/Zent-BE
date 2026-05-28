use axum::{extract::{State, Query}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait, QuerySelect, QueryOrder, Order};
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    model::responses::base::ApiResponse,
    model::responses::users::{UserResponseData, UserListResponseData},
    services::v1::users::{UserListQuery, list_users},
};

#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "users",
    params(UserListQuery),
    responses(
        (status = 200, description = "List users successful", body = ApiResponse<UserListResponseData>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn list_users_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Query(query): Query<UserListQuery>,
) -> Result<Json<ApiResponse<UserListResponseData>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let mut select = users::Entity::find();

    match current_user.role_id {
        2 => {
            // SuperAdmin: sees only Admins (1) and Technicians (4) — not Customers
            if let Some(role_name) = &query.role {
                let role_id = match role_name.to_lowercase().as_str() {
                    "admin" => 1,
                    "technician" => 4,
                    _ => return Err(AppError::BadRequest(format!("Unknown role: {}", role_name))),
                };
                select = select.filter(users::Column::RoleId.eq(role_id));
            } else {
                select = select.filter(users::Column::RoleId.is_in([1, 4]));
            }
        }
        1 => {
            // Admin: only Technicians in their province (fail-closed if no province)
            select = select.filter(users::Column::RoleId.eq(4));
            let ref province = current_user.province.as_ref().ok_or_else(|| {
                AppError::Forbidden("Admin profile missing province assignment".to_string())
            })?;
            select = select.filter(users::Column::Province.eq(province.as_str()));
        }
        _ => {
            return Err(AppError::Forbidden("Only administrators can list users".to_string()));
        }
    }

    // Exclude soft-deleted users
    select = select.filter(users::Column::DeletedAt.is_null());

    let total = select.clone().count(db.as_ref()).await?;

    let users_list = select
        .order_by(users::Column::CreatedAt, Order::Desc)
        .order_by(users::Column::Id, Order::Asc)
        .offset((page.saturating_sub(1)) * page_size)
        .limit(page_size)
        .all(db.as_ref())
        .await?;

    let effect = list_users::decide_list_users(current_user, users_list, total)?;

    Ok(Json(ApiResponse::success(200, "List users successful", UserListResponseData {
        users: effect.users.into_iter().map(|u| UserResponseData {
            id: u.id,
            role_id: u.role_id,
            full_name: u.full_name,
            email: u.email,
            phone: Some(u.phone_number),
            province: u.province,
            account_status_id: u.account_status,
            employee_id: crate::utils::user::get_employee_id(u.role_id, u.id),
            rating_counts: None,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        }).collect(),
        total: effect.total,
    })))
}

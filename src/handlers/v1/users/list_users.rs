use axum::{extract::{State, Query}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait, QuerySelect, QueryOrder, Order};
use crate::{
    core::errors::AppError,
    core::lookup_tables::LookupTables,
    entities::users,
    extractor::auth_user::AuthUser,
    infrastructure::cache::ValkeyClient,
    model::responses::base::ApiResponse,
    model::responses::users::{UserResponseData, UserListResponseData},
    services::v1::users::{UserListQuery, list_users},
    handlers::v1::work_orders as wo_handlers,
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
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Query(query): Query<UserListQuery>,
) -> Result<Json<ApiResponse<UserListResponseData>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let mut select = users::Entity::find();

    let is_tech_query = match current_user.role_id {
        2 => {
            // SuperAdmin: sees only Admins (1) and Technicians (4) — not Customers
            if let Some(role_name) = &query.role {
                let role_id = match role_name.to_lowercase().as_str() {
                    "admin" => 1,
                    "technician" => 4,
                    _ => return Err(AppError::BadRequest(format!("Unknown role: {}", role_name))),
                };
                let is_tech = role_id == 4;
                select = select.filter(users::Column::RoleId.eq(role_id));
                is_tech
            } else {
                select = select.filter(users::Column::RoleId.is_in([1, 4]));
                false // mixed roles, skip enrichment
            }
        }
        1 => {
            // Admin: only Technicians in their province (fail-closed if no province)
            select = select.filter(users::Column::RoleId.eq(4));
            let ref province = current_user.province.as_ref().ok_or_else(|| {
                AppError::Forbidden("Admin profile missing province assignment".to_string())
            })?;
            select = select.filter(users::Column::Province.eq(province.as_str()));
            true
        }
        _ => {
            return Err(AppError::Forbidden("Only administrators can list users".to_string()));
        }
    };

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

    // Resolve terminal status IDs (Closed, Rejected) for workload calculation
    let closed_id = lookup_tables.work_order_statuses_by_name.get("Closed").copied();
    let rejected_id = lookup_tables.work_order_statuses_by_name.get("Rejected").copied();
    let terminal_ids: Vec<i32> = [closed_id, rejected_id].into_iter().flatten().collect();

    let effect = list_users::decide_list_users(current_user, users_list, total)?;

    let mut response_users = Vec::with_capacity(effect.users.len());
    for u in effect.users {
        let (workload, avg_rating) = if is_tech_query && u.role_id == 4 {
            let wl = wo_handlers::get_technician_workload(db.as_ref(), &valkey_client, u.id, &terminal_ids).await;
            let stats = wo_handlers::get_cached_technician_stats(db.as_ref(), &valkey_client, u.id, &terminal_ids).await;
            let ar = stats.map(|s| s.average_rating()).unwrap_or(0.0);
            (Some(wl), Some(ar))
        } else {
            (None, None)
        };

        response_users.push(UserResponseData {
            id: u.id,
            role_id: u.role_id,
            full_name: u.full_name,
            email: u.email,
            phone: Some(u.phone_number),
            province: u.province,
            account_status_id: u.account_status,
            employee_id: crate::utils::user::get_employee_id(u.role_id, u.id),
            rating_counts: None,
            average_rating: avg_rating,
            workload,
            avatar_image_name: u.avatar_url,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        });
    }

    Ok(Json(ApiResponse::success(200, "List users successful", UserListResponseData {
        users: response_users,
        total: effect.total,
    })))
}

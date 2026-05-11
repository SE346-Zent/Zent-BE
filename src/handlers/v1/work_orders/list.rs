use axum::{extract::{State, Query}, Json};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::list_query::WorkOrderQuery;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::list_response::WorkOrderListItem;
use super::fetch_paginated_work_orders;

#[utoipa::path(
    get, path = "/api/v1/work_orders", params(WorkOrderQuery),
    responses(
        (status = 200, description = "List of work orders based on user role", body = ApiResponse<Vec<WorkOrderListItem>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Query(query): Query<WorkOrderQuery>,
) -> Result<Json<ApiResponse<Vec<WorkOrderListItem>>>, AppError> {
    if let Some(requested_role) = &query.role {
        if requested_role != &auth.role.name {
            return Err(AppError::Forbidden(format!("Requested context '{}' does not match your assigned role '{}'", requested_role, auth.role.name)));
        }
    }
    let mut resolved_province = None;
    let mut resolved_tech_id = None;
    let mut resolved_customer_id = None;

    let cache_key_prefix = match auth.role.name.as_str() {
        "SuperAdmin" => {
            resolved_province = query.province.clone();
            resolved_tech_id = query.technician_id;
            format!("superadmin:p:{:?}:t:{:?}", resolved_province, resolved_tech_id)
        }
        "Admin" => {
            let p = auth.user.province.clone().ok_or_else(|| AppError::Forbidden("Admin profile missing province".to_string()))?;
            resolved_province = Some(p.clone());
            resolved_tech_id = query.technician_id;
            format!("admin_geo:{}:t:{:?}", p, resolved_tech_id)
        }
        "Technician" => {
            resolved_tech_id = Some(auth.user.id);
            format!("tech:{}", auth.user.id)
        }
        "Customer" => {
            resolved_customer_id = Some(auth.user.id);
            format!("customer:{}", auth.user.id)
        }
        _ => return Err(AppError::Forbidden("Role not recognized".to_string())),
    };
    fetch_paginated_work_orders(db, valkey_client, lookup_tables, query.pagination, &cache_key_prefix, resolved_tech_id, resolved_province, resolved_customer_id).await
}

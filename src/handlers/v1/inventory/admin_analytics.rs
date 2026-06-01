use axum::{extract::{State, Query}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait};
use chrono::{Duration, Utc};
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::lookup_tables::LookupTables;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::inventory::analytics_query::AnalyticsQuery;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::inventory::admin_analytics_response::{AdminAnalyticsResponse, TechnicianPerformanceEntry};
use crate::services::v1::inventory::analytics::{self, AnalyticsInput, AnalyticsPeriod};
use crate::entities::{work_orders as work_orders_ent, new_part_forms, part_changes, users};

#[utoipa::path(
    get, path = "/api/v1/inventory/analytics", params(AnalyticsQuery),
    tag = "inventory",
    responses(
        (status = 200, description = "Admin analytics data", body = ApiResponse<AdminAnalyticsResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn admin_analytics(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ApiResponse<AdminAnalyticsResponse>>, AppError> {
    match auth.role.name.as_str() {
        "Admin" | "SuperAdmin" => {},
        _ => return Err(AppError::Forbidden("Only administrators can view analytics".to_string())),
    }

    let period = match query.mode.to_lowercase().as_str() {
        "weekly" | "7d" => AnalyticsPeriod::Weekly,
        "monthly" | "30d" => AnalyticsPeriod::Monthly,
        _ => return Err(AppError::BadRequest("Invalid analytics mode. Use 'weekly' or 'monthly'".to_string())),
    };

    let period_days = period.window_days();

    let now = Utc::now();
    let current_start = now - Duration::days(period_days);
    let previous_start = current_start - Duration::days(period_days);

    let closed_status_id = luts.work_order_statuses_by_name.get("Closed")
        .copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    let current_orders: Vec<chrono::DateTime<Utc>> = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::CreatedAt.gte(current_start))
        .filter(work_orders_ent::Column::CreatedAt.lt(now))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|wo| wo.created_at)
        .collect();

    let previous_orders: Vec<chrono::DateTime<Utc>> = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::CreatedAt.gte(previous_start))
        .filter(work_orders_ent::Column::CreatedAt.lt(current_start))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|wo| wo.created_at)
        .collect();

    let current_completed_orders: Vec<chrono::DateTime<Utc>> = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::WorkOrderStatusId.eq(closed_status_id))
        .filter(work_orders_ent::Column::UpdatedAt.gte(current_start))
        .filter(work_orders_ent::Column::UpdatedAt.lt(now))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|wo| wo.updated_at)
        .collect();

    let previous_completed_orders: Vec<chrono::DateTime<Utc>> = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::WorkOrderStatusId.eq(closed_status_id))
        .filter(work_orders_ent::Column::UpdatedAt.gte(previous_start))
        .filter(work_orders_ent::Column::UpdatedAt.lt(current_start))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|wo| wo.updated_at)
        .collect();

    let current_imported_parts = new_part_forms::Entity::find()
        .filter(new_part_forms::Column::DeletedAt.is_null())
        .filter(new_part_forms::Column::CreatedAt.gte(current_start))
        .filter(new_part_forms::Column::CreatedAt.lt(now))
        .count(db.as_ref())
        .await? as i64;

    let previous_imported_parts = new_part_forms::Entity::find()
        .filter(new_part_forms::Column::DeletedAt.is_null())
        .filter(new_part_forms::Column::CreatedAt.gte(previous_start))
        .filter(new_part_forms::Column::CreatedAt.lt(current_start))
        .count(db.as_ref())
        .await? as i64;

    let current_returned_parts = part_changes::Entity::find()
        .filter(part_changes::Column::DeletedAt.is_null())
        .filter(part_changes::Column::ChangeType.eq("uninstalled"))
        .filter(part_changes::Column::CreatedAt.gte(current_start))
        .filter(part_changes::Column::CreatedAt.lt(now))
        .count(db.as_ref())
        .await? as i64;

    let previous_returned_parts = part_changes::Entity::find()
        .filter(part_changes::Column::DeletedAt.is_null())
        .filter(part_changes::Column::ChangeType.eq("uninstalled"))
        .filter(part_changes::Column::CreatedAt.gte(previous_start))
        .filter(part_changes::Column::CreatedAt.lt(current_start))
        .count(db.as_ref())
        .await? as i64;

    let all_current_parts = new_part_forms::Entity::find()
        .filter(new_part_forms::Column::DeletedAt.is_null())
        .filter(new_part_forms::Column::CreatedAt.gte(current_start))
        .filter(new_part_forms::Column::CreatedAt.lt(now))
        .all(db.as_ref())
        .await?;

    let mut type_count_map: std::collections::HashMap<i32, i64> = std::collections::HashMap::new();
    for npf in &all_current_parts {
        *type_count_map.entry(npf.part_types_id).or_insert(0) += 1;
    }

    let mut part_type_counts: Vec<(String, i64)> = Vec::new();
    for (type_id, count) in type_count_map {
        let name = luts.part_types.get(&type_id).cloned().unwrap_or_else(|| format!("Type {}", type_id));
        part_type_counts.push((name, count));
    }

    let technician_role_id = *luts.roles_by_name
        .get("Technician")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Technician role missing")))?;

    let technician_models = users::Entity::find()
        .filter(users::Column::DeletedAt.is_null())
        .filter(users::Column::RoleId.eq(technician_role_id))
        .all(db.as_ref())
        .await?;

    let mut technician_performance = Vec::with_capacity(technician_models.len());
    for tech in technician_models {
        let stats = crate::handlers::v1::work_orders::get_cached_technician_stats(
            db.as_ref(),
            &valkey_client,
            tech.id,
        ).await?;

        technician_performance.push(TechnicianPerformanceEntry {
            technician_id: tech.id,
            technician_name: tech.full_name,
            total_work_orders: stats.total_work_orders,
            rating_count: stats.rating_count,
            average_rating: stats.average_rating(),
        });
    }

    technician_performance.sort_by(|a, b| b.total_work_orders.cmp(&a.total_work_orders).then_with(|| a.technician_name.cmp(&b.technician_name)));

    let input = AnalyticsInput {
        current_orders,
        previous_orders,
        current_completed_orders,
        previous_completed_orders,
        current_imported_parts,
        previous_imported_parts,
        current_returned_parts,
        previous_returned_parts,
        part_type_counts,
        technician_performance,
    };

    let response = analytics::decide_admin_analytics(input, period);
    Ok(Json(ApiResponse::success(200, "Analytics retrieved successfully", response)))
}

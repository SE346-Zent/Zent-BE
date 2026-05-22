use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, Order};
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::history_response::WorkOrderHistoryDetail;
use crate::entities::{work_orders as work_orders_ent, work_order_state_history, users, work_order_closing_forms};

/// Retrieve the full state transition history for a specific work order.

#[utoipa::path(
    get, path = "/api/v1/work_orders/{id}/history",
    responses(
        (status = 200, description = "Work order history detail (admin only)", body = ApiResponse<WorkOrderHistoryDetail>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Work order not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn history(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<WorkOrderHistoryDetail>>, AppError> {
    // Only admins can access work order history details
    match auth.role.name.as_str() {
        "Admin" | "SuperAdmin" => {},
        _ => return Err(AppError::Forbidden("Only admins can view work order history details".to_string())),
    }

    let wo = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let history_rows: Vec<(work_order_state_history::Model, Option<users::Model>)> = work_order_state_history::Entity::find()
        .filter(work_order_state_history::Column::WorkOrderId.eq(id))
        .order_by(work_order_state_history::Column::ChangedAt, Order::Asc)
        .find_also_related(users::Entity)
        .all(db.as_ref())
        .await?;

    let closing_form = if let Some(cf_id) = wo.complete_form_id {
        work_order_closing_forms::Entity::find_by_id(cf_id).one(db.as_ref()).await?
    } else {
        None
    };

    let rating = crate::entities::work_order_ratings::Entity::find()
        .filter(crate::entities::work_order_ratings::Column::WorkOrderId.eq(id))
        .one(db.as_ref())
        .await?;

    let entries = crate::services::v1::work_orders::history::decide_get_history_detail(history_rows, &luts, wo, closing_form, rating);
    Ok(Json(ApiResponse::success(200, "Work order history detail retrieved successfully", entries)))
}

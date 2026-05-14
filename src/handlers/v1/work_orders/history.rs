use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, Order};
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::history_response::WorkOrderStateHistoryEntry;
use crate::entities::{work_orders as work_orders_ent, work_order_state_history, users};

#[utoipa::path(
    get, path = "/api/v1/work_orders/{id}/history",
    responses(
        (status = 200, description = "Work order state history", body = ApiResponse<Vec<WorkOrderStateHistoryEntry>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Work order not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn history(
    _auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<WorkOrderStateHistoryEntry>>>, AppError> {
    let _ = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let history_rows: Vec<(work_order_state_history::Model, Option<users::Model>)> = work_order_state_history::Entity::find()
        .filter(work_order_state_history::Column::WorkOrderId.eq(id))
        .order_by(work_order_state_history::Column::ChangedAt, Order::Asc)
        .find_also_related(users::Entity)
        .all(db.as_ref())
        .await?;

    let entries = crate::services::v1::work_orders::history::decide_get_history(history_rows, &luts);
    Ok(Json(ApiResponse::success(200, "Work order state history retrieved successfully", entries)))
}

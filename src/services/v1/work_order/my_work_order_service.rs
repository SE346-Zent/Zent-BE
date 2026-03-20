use sea_orm::*;
use uuid::Uuid;

use crate::{
    entities::work_order,
    model::{
        responses::common::pagination::PaginationMeta,
        responses::error::AppError,
        responses::work_order::work_order_response::{
            WorkOrderListResponse, WorkOrderResponse, WorkOrderResponseData,
        },
    },
};

pub async fn get_my_work_order_service(
    db: DatabaseConnection,
    order_id: Uuid,
) -> Result<WorkOrderResponse, AppError> {
    let order_model = work_order::Entity::find_by_id(order_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("Work order not found".to_string()))?;

    let data = map_work_order(order_model);
    Ok(WorkOrderResponse::success(data))
}

pub async fn get_my_work_orders_service(
    db: DatabaseConnection,
) -> Result<WorkOrderListResponse, AppError> {
    let orders = work_order::Entity::find()
        .all(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let total_items = orders.len() as u64;
    let data = orders.into_iter().map(map_work_order).collect::<Vec<_>>();

    let meta = PaginationMeta::new(1, total_items.max(1), total_items);
    Ok(WorkOrderListResponse::success(data, meta))
}

fn map_work_order(model: work_order::Model) -> WorkOrderResponseData {
    WorkOrderResponseData {
        work_order_id: model.id,
        title: model.title,
        address_string: model.address_string,
        status_id: model.status_id,
        description: model.description,
        reject_reason: model.reject_reason,
        priority: model.priority,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        closed_at: model.closed_at.map(|d| d.to_rfc3339()),
        version: model.version,
    }
}

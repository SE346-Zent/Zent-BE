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
        requests::work_order::work_order_list_query::WorkOrderListQuery,
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
    query: WorkOrderListQuery,
) -> Result<WorkOrderListResponse, AppError> {
    let db_query = work_order::Entity::find()
        .filter(work_order::Column::AdminId.eq(query.admin_id))
        .filter(work_order::Column::CustomerId.eq(query.customer_id))
        .filter(work_order::Column::TechnicianId.eq(query.technician_id));

    let total_items = db_query
        .clone()
        .count(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let paginator = db_query.paginate(&db, query.pagination.per_page);
    let page_index = query.pagination.page.saturating_sub(1);

    let orders = paginator
        .fetch_page(page_index)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = orders.into_iter().map(map_work_order).collect::<Vec<_>>();
    let meta = PaginationMeta::new(query.pagination.page, query.pagination.per_page, total_items);
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
        admin_id: model.admin_id,
        customer_id: model.customer_id,
        technician_id: model.technician_id,
    }
}

use sea_orm::*;
use uuid::Uuid;

use crate::{
    entities::work_orders,
    model::{
        responses::common::pagination_meta::PaginationMeta,
        responses::error::AppError,
        responses::work_order::work_order_detail_response::{
            WorkOrderDetailResponse, WorkOrderDetailResponseData,
        },
        responses::work_order::work_order_list_item_response::{
            WorkOrderListResponse, WorkOrderListItemResponseData,
        },
        requests::work_order::work_order_list_query::WorkOrderListQuery,
    },
};

pub async fn get_my_work_order_service(
    db: DatabaseConnection,
    order_id: Uuid,
) -> Result<WorkOrderDetailResponse, AppError> {
    let order_model = work_orders::Entity::find_by_id(order_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("Work order not found".to_string()))?;

    let data = map_work_order_detail(order_model);
    Ok(WorkOrderDetailResponse::success(data))
}

pub async fn get_my_work_orders_service(
    db: DatabaseConnection,
    query: WorkOrderListQuery,
) -> Result<WorkOrderListResponse, AppError> {
    let db_query = work_orders::Entity::find()
        .filter(work_orders::Column::TechnicianId.eq(query.technician_id));

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

    let data = orders.into_iter().map(map_work_order_list_item).collect::<Vec<_>>();
    let meta = PaginationMeta::new(query.pagination.page, query.pagination.per_page, total_items);
    Ok(WorkOrderListResponse::success(data, meta))
}

fn map_work_order_detail(model: work_orders::Model) -> WorkOrderDetailResponseData {
    WorkOrderDetailResponseData {
        id: model.id,
        first_name: model.first_name,
        last_name: model.last_name,
        email: model.email,
        phone_number: model.phone_number,
        work_order_status_id: model.work_order_status_id,
        country: model.country,
        state: model.state,
        city: model.city,
        address: model.address,
        building: model.building,
        appointment: model.appointment.to_rfc3339(),
        reference_ticket: model.reference_ticket,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        closed_at: model.closed_at.to_rfc3339(),
        admin_id: model.admin_id,
        customer_id: model.customer_id,
        technician_id: model.technician_id,
        complete_form_id: model.complete_form_id,
        reject_form_id: model.reject_form_id,
    }
}

fn map_work_order_list_item(model: work_orders::Model) -> WorkOrderListItemResponseData {
    WorkOrderListItemResponseData {
        id: model.id,
        first_name: model.first_name,
        last_name: model.last_name,
        email: model.email,
        phone_number: model.phone_number,
        work_order_status_id: model.work_order_status_id,
        country: model.country,
        state: model.state,
        city: model.city,
        address: model.address,
        building: model.building,
        appointment: model.appointment.to_rfc3339(),
        reference_ticket: model.reference_ticket,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        closed_at: model.closed_at.to_rfc3339(),
        admin_id: model.admin_id,
        customer_id: model.customer_id,
        technician_id: model.technician_id,
        complete_form_id: model.complete_form_id,
        reject_form_id: model.reject_form_id,
    }
}

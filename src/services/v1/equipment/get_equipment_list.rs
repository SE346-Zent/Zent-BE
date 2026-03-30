use sea_orm::*;
use uuid::Uuid;

use crate:: {
    entities::equipment,
    model:: {
        requests::equipment::equipment_list_query::EquipmentListQuery,
        responses::error::AppError,
        responses::equipment::equipment_list_item_response::{EquipmentListResponse, EquipmentListItemResponseData}
    }
};

pub async fn get_equipment_list_service(
    db: DatabaseConnection,
    query: EquipmentListQuery
) -> Result<EquipmentListResponse, AppError> {
    let db_query = equipment::Entity::find()
        .filter(equipment::Column::CustomerId.eq(query.customer_id));

    let total_items = db_query
        .clone()
        .count(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let paginator = db_query.paginate(&db, query.pagination.per_page);
    let page_index = query.pagination.page.saturating_sub(1);

    let equipments = paginator
        .fetch_page(page_index)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = equipments.into_iter().map(map_equipment_list_item).collect::<Vec<_>>();
    let meta = PaginationMeta::new(query.pagination.page, query.pagination.per_page, total_items);
    Ok(EquipmentListResponse::success(data, meta))
}

fn map_equipment_list_item(model: equipment::Model) -> EquipmentListItemResponseData {
    EquipmentListItemResponseData {
        id: model.id,
        equipment_status_id: model.equipment_status_id,
        model_id: model.model_id,
        customer_id: model.customer_id,
        equipment_name: model.equipment_name,
        serial_number: model.serial_number,
        created_at: model.created_at.to_string(),
        updated_at: model.updated_at.to_string(),
        deleted_at: model.deleted_at.map(|d| d.to_string()),
    }
}
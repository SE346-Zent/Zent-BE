use seaorm::*;
use uuid::Uuid;

use crate::{
    entities::parts,
    requests::part::part_list_query::PartListQuery,
    responses::error::AppError,
    responses::part::part_list_response::{PartListResponse, PartListItemResponseData},
    responses::pagination::PaginationMeta,
};

pub async fn get_part_list_service(
    db: DatabaseConnection,
    query: PartListQuery
) -> Result<PartListResponse, AppError> {
    let db_query = parts::Entity::find()
        .filter(parts::Column::CustomerId.eq(query.customer_id));

    let total_items = db_query
        .clone()
        .count(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = db_query
        .clone()
        .limit(query.page_size)
        .offset(query.page_size * (query.page - 1))
        .all(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let meta = PaginationMeta {
        page: query.page,
        page_size: query.page_size,
        total_items,
        total_pages: (total_items as f64 / query.page_size as f64).ceil() as i32,
    };

    let data = data.into_iter().map(map_part_list_item).collect();
    Ok(PartListResponse::success(data, meta))
}

fn map_part_list_item(model: parts::Model) -> PartListItemResponseData {
    PartListItemResponseData {
        part_id: model.id,
        part_name: model.part_name,
        part_status_id: model.part_status_id,
        part_type_id: model.part_type_id,
        quantity: model.quantity,
        serial_number: model.serial_number,
        last_modified_at: model.last_modified_at.map(|d| d.to_rfc3339()),
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        deleted_at: model.deleted_at.map(|d| d.to_rfc3339()),
    }
}
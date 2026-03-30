use sea_orm::*;
use uuid::Uuid;

use crate::{
    entities::parts,
    requests::part::part_detail_query::PartDetailQuery,
    responses::error::AppError,
    responses::part::part_detail_response::PartDetailResponse,
};

pub async fn get_part_detail_service(
    db: DatabaseConnection,
    query: PartDetailQuery
) -> Result<PartDetailResponse, AppError> {
    let part_model = parts::Entity::find_by_id(query.part_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::NotFound(anyhow::anyhow!("Part not found!")));

    let data = map_part_detail(part_model);
    Ok(PartDetailResponse::success(data))
}

fn map_part_detail(model: parts::Model) -> PartDetailResponseData {
    PartDetailResponseData {
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
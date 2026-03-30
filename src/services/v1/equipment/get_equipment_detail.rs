use sea_orm::*;
use uuid::Uuid;

use crate:: {
    entities::{equipment},
    model::{
        requests::equipment::equipment_detail_query::EquipmentDetailQuery,
        responses::error::AppError,
        responses::equipment::equipment_detail_response::{EquipmentDetailResponse, EquipmentDetailResponseData}
    }
};

pub async fn get_equipment_detail_service(
    db: DatabaseConnection,
    query: EquipmentDetailQuery
) -> Result<EquipmentDetailResponse, AppError> {
    let equipment_model = equipment::Entity::find_by_id(query.equipment_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("Equipment not found".to_string()))?;

    let data = EquipmentDetailResponseData {
        id: equipment_model.id,
        equipment_status_id: equipment_model.equipment_status_id,
        model_id: equipment_model.model_id,
        customer_id: equipment_model.customer_id,
        equipment_name: equipment_model.equipment_name,
        serial_number: equipment_model.serial_number,
        created_at: equipment_model.created_at.to_string(),
        updated_at: equipment_model.updated_at.to_string(),
        deleted_at: equipment_model.deleted_at.map(|d| d.to_string()),
    };

    Ok(EquipmentDetailResponse::success(data))
}
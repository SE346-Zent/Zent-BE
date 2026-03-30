use sea_orm::*;
use uuid::Uuid;

use crate::{
    entities::warranties,
    model::responses::{
        error::AppError,
        warranty::warranty_detail_response::{WarrantyDetailResponse, WarrantyDetailResponseData},
    },
};

pub async fn get_warranty_service(
    db: DatabaseConnection,
    warranty_id: Uuid,
) -> Result<WarrantyDetailResponse, AppError> {
    let warranty_model = warranties::Entity::find_by_id(warranty_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("Warranty not found".to_string()))?;

    let data = WarrantyDetailResponseData {
        id: warranty_model.id,
        customer_id: warranty_model.customer_id,
        equipment_id: warranty_model.equipment_id,
        start_date: warranty_model.start_date.to_rfc3339(),
        end_date: warranty_model.end_date.map(|d| d.to_rfc3339()),
        warranty_status: warranty_model.warranty_status,
        created_at: warranty_model.created_at.to_rfc3339(),
        updated_at: warranty_model.updated_at.to_rfc3339(),
        deleted_at: warranty_model.deleted_at.map(|d| d.to_rfc3339()),
    };

    Ok(WarrantyDetailResponse::success(data))
}

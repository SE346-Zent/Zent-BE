use sea_orm::*;
use uuid::Uuid;

use crate:: {
    entities::{products},
    model::{
        requests::product::product_detail_query::ProductDetailQuery,
        responses::error::AppError,
        responses::product::product_detail_response::{ProductDetailResponse, ProductDetailResponseData}
    }
};

pub async fn get_product_detail_service(
    db: DatabaseConnection,
    query: ProductDetailQuery
) -> Result<ProductDetailResponse, AppError> {
    let product_model = products::Entity::find_by_id(query.product_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("Product not found".to_string()))?;

    let data = ProductDetailResponseData {
        id: product_model.id,
        model_id: product_model.model_id,
        customer_id: product_model.customer_id,
        product_name: product_model.product_name,
        serial_number: product_model.serial_number,
        created_at: product_model.created_at.to_string(),
        updated_at: product_model.updated_at.to_string(),
        deleted_at: product_model.deleted_at.map(|d| d.to_string()),
    };

    Ok(ProductDetailResponse::success(data))
}

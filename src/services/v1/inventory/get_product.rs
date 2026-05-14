use crate::core::errors::AppError;
use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions};
use crate::model::responses::inventory::product_detail_response::ProductDetailResponse;
use crate::model::responses::inventory::part_list_item::PartListItem;

pub struct ProductWithRelations {
    pub product: prod::Model,
    pub model: product_models::Model,
    pub parts: Vec<PartInProduct>,
}

pub struct PartInProduct {
    pub part: parts::Model,
    pub catalog: part_catalog::Model,
    pub condition: part_conditions::Model,
    pub status: String,
    pub technician_id: Option<uuid::Uuid>,
}

fn can_user_see(role_name: &str, user_id: uuid::Uuid, p: &ProductWithRelations) -> bool {
    match role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => p.parts.iter().any(|rp| rp.technician_id == Some(user_id)),
        "customer" => p.product.customer_id == user_id,
        _ => false,
    }
}

pub fn get_product_detail(
    p: &ProductWithRelations,
    role_name: &str,
    user_id: uuid::Uuid,
) -> Result<ProductDetailResponse, AppError> {
    if !can_user_see(role_name, user_id, p) {
        return Err(AppError::Forbidden("You do not have access to this product".to_string()));
    }
    Ok(ProductDetailResponse {
        product_id: p.product.id,
        product_name: p.product.product_name.clone(),
        model_code: p.product.product_model_code.clone(),
        model_name: p.model.model_name.clone(),
        serial_number: p.product.serial_number.clone(),
        customer_id: p.product.customer_id,
        customer_name: format!("Customer {}", p.product.customer_id),
        parts: p.parts.iter().map(|rp| PartListItem {
            part_id: rp.part.id,
            part_number: rp.catalog.part_number.clone(),
            part_type_name: rp.catalog.part_types_id.to_string(),
            serial_number: rp.part.serial_number.clone(),
            condition_name: rp.condition.name.clone(),
            product_name: Some(p.product.product_name.clone()),
            approval_status: rp.status.clone(),
            created_at: rp.part.created_at.to_rfc3339(),
        }).collect(),
        created_at: p.product.created_at.to_rfc3339(),
        updated_at: p.product.updated_at.to_rfc3339(),
    })
}

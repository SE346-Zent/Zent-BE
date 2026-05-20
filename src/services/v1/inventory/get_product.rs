use crate::core::errors::AppError;
use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions};
use crate::model::responses::inventory::product_detail_response::ProductDetailResponse;
use crate::model::responses::inventory::part_list_item::PartListItem;

/// Represents a single product joined with its related model and installed parts data.
pub struct ProductWithRelations {
    /// The core product record.
    pub product_record: prod::Model,
    /// The product model definition.
    pub model_definition: product_models::Model,
    /// A list of parts currently installed in this product.
    pub installed_parts: Vec<PartInProduct>,
}

/// Represents a part associated with a product, including its catalog, condition, and status data.
pub struct PartInProduct {
    /// The core part record.
    pub part_record: parts::Model,
    /// The associated catalog definition.
    pub catalog_definition: part_catalog::Model,
    /// The current physical condition of the part.
    pub physical_condition: part_conditions::Model,
    /// The current approval status of the part.
    pub approval_status: String,
    /// The ID of the technician who registered this part.
    pub registering_technician_id: Option<uuid::Uuid>,
}

/// Determine if a user with a specific role is permitted to see the details of a particular product.
///
/// Visibility rules:
/// - Admins and Managers can see all products.
/// - Technicians can see products if they registered at least one part currently in it.
/// - Customers can see products they registered.
fn can_user_see_product_detail(
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
    product_relation_data: &ProductWithRelations,
) -> bool {
    match requesting_role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => product_relation_data.installed_parts.iter().any(|part| part.registering_technician_id == Some(requesting_user_id)),
        "customer" => product_relation_data.product_record.customer_id == requesting_user_id,
        _ => false,
    }
}

/// Assemble detailed information for a single product, filtered by user visibility rules.
///
/// This function converts the joined database data into a response model,
/// including a list of all parts currently installed in the product.
///
/// # Arguments
/// * `product_relation_data` - The assembled product data including model and installed parts.
/// * `requesting_role_name` - The role of the user requesting the details.
/// * `requesting_user_id` - The unique identifier of the requesting user.
///
/// # Returns
/// A result containing the `ProductDetailResponse` on success, or a `Forbidden` error if access is denied.
pub fn get_product_detail(
    product_relation_data: &ProductWithRelations,
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
) -> Result<ProductDetailResponse, AppError> {
    if !can_user_see_product_detail(requesting_role_name, requesting_user_id, product_relation_data) {
        return Err(AppError::Forbidden("You do not have access to this product".to_string()));
    }
    Ok(ProductDetailResponse {
        product_id: product_relation_data.product_record.id,
        product_name: product_relation_data.product_record.product_name.clone(),
        model_code: product_relation_data.product_record.product_model_code.clone(),
        model_name: product_relation_data.model_definition.model_name.clone(),
        serial_number: product_relation_data.product_record.serial_number.clone(),
        customer_id: product_relation_data.product_record.customer_id,
        customer_name: format!("Customer {}", product_relation_data.product_record.customer_id),
        parts: product_relation_data.installed_parts.iter().map(|installed_part| PartListItem {
            part_id: installed_part.part_record.id,
            part_number: installed_part.catalog_definition.part_number.clone(),
            part_type_name: installed_part.catalog_definition.part_types_id.to_string(),
            serial_number: installed_part.part_record.serial_number.clone(),
            condition_name: installed_part.physical_condition.name.clone(),
            product_name: Some(product_relation_data.product_record.product_name.clone()),
            approval_status: installed_part.approval_status.clone(),
            created_at: installed_part.part_record.created_at.to_rfc3339(),
        }).collect(),
        created_at: product_relation_data.product_record.created_at.to_rfc3339(),
        updated_at: product_relation_data.product_record.updated_at.to_rfc3339(),
    })
}

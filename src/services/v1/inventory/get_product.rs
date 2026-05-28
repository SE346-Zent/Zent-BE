use crate::core::errors::AppError;
use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions};
use crate::model::responses::inventory::product_detail_response::{ProductDetailResponse, ProductWarrantySummary, ProductWorkOrderHistoryItem};

/// Represents a single product joined with its related model and installed parts data.
pub struct ProductWithRelations {
    /// The core product record.
    pub product_record: prod::Model,
    /// The product model definition.
    pub model_definition: product_models::Model,
    /// The model image URL resolved from SCM.
    pub product_image_url: Option<String>,
    /// A list of parts currently installed in this product.
    pub installed_parts: Vec<PartInProduct>,
    /// Warranty summary for the product, if any.
    pub warranty: Option<ProductWarrantySummary>,
    /// Recent work orders for the product.
    pub work_order_history: Vec<ProductWorkOrderHistoryItem>,
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
        title: product_relation_data.product_record.product_name.clone(),
        model_code: product_relation_data.product_record.product_model_code.clone(),
        model_name: product_relation_data.model_definition.model_name.clone(),
        product_image_url: product_relation_data.product_image_url.clone(),
        serial_number: product_relation_data.product_record.serial_number.clone(),
        warranty: product_relation_data.warranty.clone(),
        work_order_history: product_relation_data.work_order_history.clone(),
        created_at: product_relation_data.product_record.created_at.to_rfc3339(),
        updated_at: product_relation_data.product_record.updated_at.to_rfc3339(),
    })
}

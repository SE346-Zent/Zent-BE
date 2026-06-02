use crate::core::errors::AppError;
use crate::entities::{parts, part_catalog, part_conditions, products};
use crate::model::responses::inventory::part_detail_response::PartDetailResponse;

/// Represents a single part joined with its related catalog, condition, and product data.
pub struct PartWithRelations {
    /// The core part record.
    pub part_record: parts::Model,
    /// The associated catalog definition.
    pub catalog_definition: part_catalog::Model,
    /// The current physical condition of the part.
    pub physical_condition: part_conditions::Model,
    /// The product this part is currently installed in, if any.
    pub installed_product: Option<products::Model>,
    /// The current approval status string.
    pub approval_status: String,
    /// Optional reason if the part addition was denied.
    pub denial_reason: Option<String>,
    /// The ID of the customer who owns the product this part is in.
    pub customer_id: Option<uuid::Uuid>,
    /// The ID of the technician who registered this part.
    pub technician_id: Option<uuid::Uuid>,
}

/// Determine if a user with a specific role is permitted to see the details of a particular part.
///
/// Visibility rules:
/// - Admins and Managers can see all parts.
/// - Technicians can see parts they registered.
/// - Customers can see approved parts belonging to their products.
fn can_user_see_part_detail(
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
    part_relation_data: &PartWithRelations,
) -> bool {
    match requesting_role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => part_relation_data.technician_id == Some(requesting_user_id),
        "customer" => {
            part_relation_data.approval_status == "approved" 
                && part_relation_data.customer_id == Some(requesting_user_id)
        }
        _ => false,
    }
}

/// Assemble detailed information for a single part, filtered by user visibility rules.
///
/// This function converts the joined database data into a response model,
/// ensuring that the requesting user is permitted to see the details of this
/// particular part.
///
/// # Arguments
/// * `part_relation_data` - The assembled part data including catalog and installed product info.
/// * `requesting_role_name` - The role of the user requesting the details.
/// * `requesting_user_id` - The unique identifier of the requesting user.
///
/// # Returns
/// A result containing the `PartDetailResponse` on success, or a `Forbidden` error if access is denied.
pub fn get_part_detail(
    part_relation_data: &PartWithRelations,
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
) -> Result<PartDetailResponse, AppError> {
    if !can_user_see_part_detail(requesting_role_name, requesting_user_id, part_relation_data) {
        tracing::warn!(
            error.message = "NotAuthorized", error.details = "",
            part_id = %part_relation_data.part_record.id,
            requesting_role_name = %requesting_role_name,
            requesting_user_id = %requesting_user_id,
            message = "User is not authorized to see part details"
        );
        return Err(AppError::Forbidden("You do not have access to this part".to_string()));
    }
    Ok(PartDetailResponse {
        part_id: part_relation_data.part_record.id,
        part_number: part_relation_data.catalog_definition.part_number.clone(),
        part_type_id: part_relation_data.catalog_definition.part_types_id,
        part_type_name: part_relation_data.catalog_definition.part_types_id.to_string(),
        model_code: None,
        serial_number: part_relation_data.part_record.serial_number.clone(),
        description: part_relation_data.catalog_definition.description.clone(),
        condition_id: part_relation_data.physical_condition.id,
        condition_name: part_relation_data.physical_condition.name.clone(),
        product_id: part_relation_data.installed_product.as_ref().map(|product| product.id),
        product_name: part_relation_data.installed_product.as_ref().map(|product| product.product_name.clone()),
        manufactured_date: Some(part_relation_data.part_record.manufactured_date.to_rfc3339()),
        installation_date: part_relation_data.part_record.installation_date.map(|timestamp| timestamp.to_rfc3339()),
        approval_status: part_relation_data.approval_status.clone(),
        denial_reason: part_relation_data.denial_reason.clone(),
        created_at: part_relation_data.part_record.created_at.to_rfc3339(),
        updated_at: part_relation_data.part_record.updated_at.to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::TimeZone;

    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
    fn t() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&chrono::NaiveDateTime::parse_from_str("2024-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap())
    }

    #[test]
    fn test_admin_can_get_detail() {
        let part_relation_data = PartWithRelations {
            part_record: parts::Model { id: u("10000000-0000-0000-0000-000000000000"), part_catalog_id: u("00000000-0000-0000-0000-000000000000"), product_id: None, serial_number: "SN-1".into(), part_condition_id: 1, manufactured_date: t(), installation_date: None, removal_date: None, scrapped_date: None, created_at: t(), updated_at: t(), deleted_at: None },
            catalog_definition: part_catalog::Model { id: u("00000000-0000-0000-0000-000000000000"), part_number: "PN-001".into(), part_types_id: 1, mfg_number: "MFG".into(), description: None, part_mfg_status: 1, created_at: t(), updated_at: t(), deleted_at: None },
            physical_condition: part_conditions::Model { id: 1, name: "New".into() },
            installed_product: None, 
            approval_status: "pending".into(), 
            denial_reason: None,
            customer_id: None, 
            technician_id: Some(u("b0000000-0000-0000-0000-000000000000")),
        };
        let result = get_part_detail(&part_relation_data, "Admin", u("a0000000-0000-0000-0000-000000000000"));
        assert!(result.is_ok());
    }
}

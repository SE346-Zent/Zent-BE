use crate::entities::warranties;
use crate::model::responses::inventory::verify_product_response::VerifyProductResponse;
use uuid::Uuid;

/// Verify a product by serial number and check if it has been registered.
///
/// This function checks:
/// 1. If the product exists in the SCM catalog
/// 2. If the product has been registered by any customer
/// 3. If the product is still under warranty
///
/// # Arguments
/// * `product_id` - The product ID from SCM
/// * `serial_number` - The serial number to verify
/// * `product_name` - The product name from SCM
/// * `product_model_code` - The model code from SCM
/// * `is_registered` - Whether the product is already registered
/// * `existing_warranty` - Optional warranty record for the product
/// * `current_time` - Current timestamp for warranty validation
///
/// # Returns
/// A `VerifyProductResponse` with the verification result
pub fn determine_verify_product_result(
    product_id: Uuid,
    serial_number: &str,
    product_name: &str,
    product_model_code: &str,
    is_registered: bool,
    existing_warranty: Option<warranties::Model>,
    current_time: chrono::DateTime<chrono::Utc>,
) -> VerifyProductResponse {
    let (warranty_status, warranty_start_date, warranty_end_date) = match &existing_warranty {
        Some(w) => {
            if current_time > w.end_date {
                ("expired".to_string(), Some(w.start_date.to_rfc3339()), Some(w.end_date.to_rfc3339()))
            } else {
                (w.warranty_status.clone(), Some(w.start_date.to_rfc3339()), Some(w.end_date.to_rfc3339()))
            }
        }
        None => ("none".to_string(), None, None),
    };

    let is_warranty_valid = warranty_status == "active";

    if !is_warranty_valid {
        return VerifyProductResponse {
            product_id,
            serial_number: serial_number.to_string(),
            product_name: product_name.to_string(),
            product_model_code: product_model_code.to_string(),
            is_registered,
            warranty_status,
            warranty_start_date,
            warranty_end_date,
            message: "Product cannot be verified: warranty is expired or not available".to_string(),
        };
    }

    if is_registered {
        VerifyProductResponse {
            product_id,
            serial_number: serial_number.to_string(),
            product_name: product_name.to_string(),
            product_model_code: product_model_code.to_string(),
            is_registered: true,
            warranty_status,
            warranty_start_date,
            warranty_end_date,
            message: "This product has already been registered".to_string(),
        }
    } else {
        VerifyProductResponse {
            product_id,
            serial_number: serial_number.to_string(),
            product_name: product_name.to_string(),
            product_model_code: product_model_code.to_string(),
            is_registered: false,
            warranty_status,
            warranty_start_date,
            warranty_end_date,
            message: "Product is available for registration".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }

    fn active_warranty(product_id: Uuid) -> warranties::Model {
        let now = chrono::Utc::now();
        warranties::Model {
            id: u("00000000-0000-0000-0000-000000000001"),
            customer_id: u("00000000-0000-0000-0000-000000000002"),
            product_id,
            start_date: now - chrono::Duration::days(10),
            end_date: now + chrono::Duration::days(355),
            warranty_status: "active".to_string(),
            warranty_status_id: Some(1),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    fn expired_warranty(product_id: Uuid) -> warranties::Model {
        let now = chrono::Utc::now();
        warranties::Model {
            id: u("00000000-0000-0000-0000-000000000001"),
            customer_id: u("00000000-0000-0000-0000-000000000002"),
            product_id,
            start_date: now - chrono::Duration::days(400),
            end_date: now - chrono::Duration::days(10),
            warranty_status: "active".to_string(),
            warranty_status_id: Some(1),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    #[test]
    fn test_product_not_registered_active_warranty() {
        let now = chrono::Utc::now();
        let prod_id = u("00000000-0000-0000-0000-000000000003");
        let warranty = active_warranty(prod_id);
        let result = determine_verify_product_result(
            prod_id, "SN-001", "ThinkPad X1", "MOD-A", false, Some(warranty), now,
        );
        assert!(!result.is_registered);
        assert_eq!(result.warranty_status, "active");
        assert_eq!(result.message, "Product is available for registration");
    }

    #[test]
    fn test_product_registered_active_warranty() {
        let now = chrono::Utc::now();
        let prod_id = u("00000000-0000-0000-0000-000000000003");
        let warranty = active_warranty(prod_id);
        let result = determine_verify_product_result(
            prod_id, "SN-001", "ThinkPad X1", "MOD-A", true, Some(warranty), now,
        );
        assert!(result.is_registered);
        assert_eq!(result.warranty_status, "active");
        assert_eq!(result.message, "This product has already been registered");
    }

    #[test]
    fn test_product_expired_warranty() {
        let now = chrono::Utc::now();
        let prod_id = u("00000000-0000-0000-0000-000000000003");
        let warranty = expired_warranty(prod_id);
        let result = determine_verify_product_result(
            prod_id, "SN-001", "ThinkPad X1", "MOD-A", false, Some(warranty), now,
        );
        assert_eq!(result.warranty_status, "expired");
        assert_eq!(result.message, "Product cannot be verified: warranty is expired or not available");
    }

    #[test]
    fn test_product_no_warranty() {
        let now = chrono::Utc::now();
        let prod_id = u("00000000-0000-0000-0000-000000000003");
        let result = determine_verify_product_result(
            prod_id, "SN-001", "ThinkPad X1", "MOD-A", false, None, now,
        );
        assert_eq!(result.warranty_status, "none");
        assert_eq!(result.message, "Product cannot be verified: warranty is expired or not available");
    }
}

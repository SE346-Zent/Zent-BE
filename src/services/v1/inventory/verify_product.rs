use crate::model::responses::inventory::verify_product_response::VerifyProductResponse;
use uuid::Uuid;

/// Verify a product by serial number and check if it has been registered.
///
/// This function checks:
/// 1. If the product exists in the SCM catalog
/// 2. If the product has been registered by any customer
///
/// # Arguments
/// * `product_id` - The product ID from SCM
/// * `serial_number` - The serial number to verify
/// * `product_name` - The product name from SCM
/// * `product_model_code` - The model code from SCM
/// * `is_registered` - Whether the product is already registered
///
/// # Returns
/// A `VerifyProductResponse` with the verification result
pub fn determine_verify_product_result(
    product_id: Uuid,
    serial_number: &str,
    product_name: &str,
    product_model_code: &str,
    is_registered: bool,
) -> VerifyProductResponse {
    if is_registered {
        VerifyProductResponse {
            product_id,
            serial_number: serial_number.to_string(),
            product_name: product_name.to_string(),
            product_model_code: product_model_code.to_string(),
            is_registered: true,
            message: "This product has already been registered".to_string(),
        }
    } else {
        VerifyProductResponse {
            product_id,
            serial_number: serial_number.to_string(),
            product_name: product_name.to_string(),
            product_model_code: product_model_code.to_string(),
            is_registered: false,
            message: "Product is available for registration".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_not_registered() {
        let result = determine_verify_product_result(
            Uuid::new_v4(),
            "SN-001",
            "ThinkPad X1",
            "MOD-A",
            false,
        );
        assert!(!result.is_registered);
        assert_eq!(result.message, "Product is available for registration");
    }

    #[test]
    fn test_product_already_registered() {
        let result = determine_verify_product_result(
            Uuid::new_v4(),
            "SN-001",
            "ThinkPad X1",
            "MOD-A",
            true,
        );
        assert!(result.is_registered);
        assert_eq!(result.message, "This product has already been registered");
    }
}

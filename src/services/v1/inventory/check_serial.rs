use crate::services::v1::inventory::ports::ZeusProduct;

/// Check whether a specific serial number exists in the Zeus product catalog.
///
/// This pure function performs a case-insensitive, whitespace-trimmed comparison
/// to ensure robust validation of a product serial number against a found product.
///
/// # Arguments
/// * `provided_serial_number` - The serial number string to be validated.
/// * `zeus_product` - An optional product found in Zeus via serial lookup.
///
/// # Returns
/// `true` if a matching product is found with a matching serial, `false` otherwise.
pub fn check_serial_exists(
    provided_serial_number: &str,
    zeus_product: &Option<ZeusProduct>,
) -> bool {
    match zeus_product {
        Some(product) => {
            let normalized_input = provided_serial_number.trim().to_lowercase();
            product.serial_number.to_lowercase() == normalized_input
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn mock_product(serial: &str) -> ZeusProduct {
        let now = chrono::Utc::now();
        ZeusProduct {
            id: Uuid::new_v4(),
            product_model_code: "MOD-A".to_string(),
            customer_id: Uuid::new_v4(),
            product_name: "Product A".to_string(),
            serial_number: serial.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_finds_match() {
        let product = mock_product("SN-001");
        assert!(check_serial_exists("SN-001", &Some(product)));
    }

    #[test]
    fn test_no_match() {
        let product = mock_product("SN-001");
        assert!(!check_serial_exists("SN-999", &Some(product)));
    }

    #[test]
    fn test_case_insensitive() {
        let product = mock_product("SN-001");
        assert!(check_serial_exists("sn-001", &Some(product)));
    }

    #[test]
    fn test_trims_whitespace() {
        let product = mock_product("SN-001");
        assert!(check_serial_exists("  SN-001  ", &Some(product)));
    }

    #[test]
    fn test_none_product() {
        assert!(!check_serial_exists("SN-001", &None));
    }
}

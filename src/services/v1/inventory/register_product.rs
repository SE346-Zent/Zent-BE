use crate::core::errors::AppError;
use crate::model::requests::inventory::register_product_request::RegisterProductRequest;

/// Represents the calculated results and side-effects of a successful product registration request.
pub struct RegisterProductEffect {
    /// The unique identifier assigned to the registered product.
    pub registered_product_id: uuid::Uuid,
    /// The unique identifier of the customer who registered the product.
    pub customer_id: uuid::Uuid,
    /// The formatted name of the product (includes model, country, and year).
    pub product_display_name: String,
    /// The model code identified from the product catalog.
    pub product_model_code: String,
    /// The product's unique serial number.
    pub product_serial_number: String,
    /// Boolean indicating if a registration confirmation email should be sent.
    pub should_send_confirmation_email: bool,
    /// The email address to which the confirmation will be sent.
    pub customer_email_address: String,
    /// The full name of the customer for email personalization.
    pub customer_full_name: String,
}

/// Determine the outcome of a product registration request by validating serial existence and handling re-registration.
///
/// This pure function checks the provided serial number against the catalog and
/// handles the logic for both new registrations and re-registration of 
/// previously registered products (which does not trigger a confirmation email).
///
/// # Arguments
/// * `registration_payload` - The request containing product and customer details.
/// * `requesting_user_id` - The unique identifier of the customer performing registration.
/// * `customer_full_name` - The user's name for personalization and record-keeping.
/// * `catalog_model_code` - The model code retrieved from the internal serial catalog.
/// * `catalog_model_name` - The product model name retrieved from the internal serial catalog.
/// * `existing_product_record_id` - The ID of an existing product record with the same serial, if any.
/// * `current_timestamp` - The current time used for record naming and timestamps.
///
/// # Returns
/// A result containing the `RegisterProductEffect` on success, or an `AppError` if the serial is invalid.
pub fn decide_register_product(
    registration_payload: &RegisterProductRequest,
    requesting_user_id: uuid::Uuid,
    customer_full_name: &str,
    catalog_model_code: Option<String>,
    catalog_model_name: Option<String>,
    existing_product_record_id: Option<uuid::Uuid>,
    current_timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<RegisterProductEffect, AppError> {
    let identified_model_code = catalog_model_code
        .ok_or_else(|| AppError::BadRequest(format!("Serial number '{}' not found in the product catalog", registration_payload.serial_number)))?;
    let identified_model_name = catalog_model_name
        .ok_or_else(|| AppError::BadRequest(format!("Model name not found for serial '{}'", registration_payload.serial_number)))?;

    if let Some(product_id) = existing_product_record_id {
        // Re-registration — returns existing ID, no email
        let effective_country = if registration_payload.country.is_empty() { "Vietnam".to_string() } else { registration_payload.country.clone() };
        return Ok(RegisterProductEffect {
            registered_product_id: product_id,
            customer_id: requesting_user_id,
            product_display_name: format!("{} {} {}", identified_model_name, effective_country, current_timestamp.format("%Y")),
            product_model_code: identified_model_code,
            product_serial_number: registration_payload.serial_number.clone(),
            should_send_confirmation_email: false,
            customer_email_address: registration_payload.email.clone(),
            customer_full_name: customer_full_name.to_string(),
        });
    }

    let effective_country = if registration_payload.country.is_empty() { "Vietnam".to_string() } else { registration_payload.country.clone() };
    Ok(RegisterProductEffect {
        registered_product_id: uuid::Uuid::new_v4(),
        customer_id: requesting_user_id,
        product_display_name: format!("{} {} {}", identified_model_name, effective_country, current_timestamp.format("%Y")),
        product_model_code: identified_model_code,
        product_serial_number: registration_payload.serial_number.clone(),
        should_send_confirmation_email: registration_payload.send_email_confirmation && !registration_payload.email.is_empty(),
        customer_email_address: registration_payload.email.clone(),
        customer_full_name: customer_full_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
    fn req() -> RegisterProductRequest {
        RegisterProductRequest {
            serial_number: "SN-001".into(), country: "Vietnam".into(), province: "HN".into(),
            city: "HN".into(), address: "123 St".into(), first_name: "A".into(), last_name: "B".into(),
            email: "a@b.com".into(), mobile_phone: "0912345678".into(), send_email_confirmation: true,
        }
    }

    #[test]
    fn test_success() {
        let result = decide_register_product(&req(), u("c0000000-0000-0000-0000-000000000001"), "Alice",
            Some("MOD-A".into()), Some("ThinkPad".into()), None, chrono::Utc::now());
        assert!(result.is_ok());
        let effect = result.unwrap();
        assert!(effect.should_send_confirmation_email);
    }

    #[test]
    fn test_unknown_serial() {
        let result = decide_register_product(&req(), u("c0000000-0000-0000-0000-000000000001"), "Alice",
            None, None, None, chrono::Utc::now());
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_returns_no_email() {
        let result = decide_register_product(&req(), u("c0000000-0000-0000-0000-000000000001"), "Alice",
            Some("MOD-A".into()), Some("ThinkPad".into()),
            Some(u("10000000-0000-0000-0000-000000000001")), chrono::Utc::now());
        assert!(result.is_ok());
        assert!(!result.unwrap().should_send_confirmation_email);
    }
}

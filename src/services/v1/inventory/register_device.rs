use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::registered_devices,
    model::requests::inventory::register_device_request::RegisterDeviceRequest,
};

/// Represents the calculated results and side-effects of a successful device registration request.
#[derive(Debug)]
pub struct RegisterDeviceEffect {
    /// The unique identifier assigned to the registration record.
    pub registration_id: Uuid,
    /// The unique identifier of the customer who registered the device.
    pub customer_id: Uuid,
    /// The unique identifier of the registered product.
    pub product_id: Uuid,
    /// The product's unique serial number.
    pub product_serial_number: String,
    /// The model code identified from the product catalog.
    pub product_model_code: String,
    /// The product display name.
    pub product_display_name: String,
    /// The country of registration (always "Vietnam").
    pub country: String,
    /// The province of registration.
    pub province: String,
    /// The ward of registration.
    pub ward: String,
    /// The address of registration.
    pub address: String,
    /// Customer's first name.
    pub first_name: String,
    /// Customer's last name.
    pub last_name: String,
    /// Customer's email address.
    pub email: String,
    /// Customer's mobile phone number.
    pub mobile_phone: String,
    /// Boolean indicating if a registration confirmation email should be sent.
    pub should_send_confirmation_email: bool,
    /// The full name of the customer for email personalization.
    pub customer_full_name: String,
}

/// Determine the outcome of a device registration request by validating province and preparing data.
///
/// This pure function validates the province (must be HN or HCM), ensures the country is Vietnam,
/// and prepares the models for the registration record.
///
/// # Arguments
/// * `registration_payload` - The request containing device and customer details.
/// * `requesting_user_id` - The unique identifier of the customer performing registration.
/// * `customer_full_name` - The user's name for personalization and record-keeping.
/// * `product_id` - The ID of the product being registered.
/// * `catalog_model_code` - The model code retrieved from the internal serial catalog.
/// * `catalog_model_name` - The product model name retrieved from the internal serial catalog.
/// * `current_timestamp` - The current time used for record naming and timestamps.
///
/// # Returns
/// A result containing the `RegisterDeviceEffect` on success, or an `AppError` if validation fails.
pub fn decide_register_device(
    registration_payload: &RegisterDeviceRequest,
    requesting_user_id: Uuid,
    customer_full_name: &str,
    product_id: Uuid,
    catalog_model_code: String,
    catalog_model_name: String,
    current_timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<RegisterDeviceEffect, AppError> {
    // Validate province (must be HN or HCM)
    let province = registration_payload.province.to_uppercase();
    if province != "HN" && province != "HCM" {
        return Err(AppError::BadRequest("Registration is only available in Hanoi (HN) or Ho Chi Minh City (HCM)".to_string()));
    }

    // Country is always Vietnam
    let country = "Vietnam".to_string();

    Ok(RegisterDeviceEffect {
        registration_id: Uuid::new_v4(),
        customer_id: requesting_user_id,
        product_id,
        product_serial_number: registration_payload.serial_number.clone(),
        product_model_code: catalog_model_code,
        product_display_name: format!("{} {} {}", catalog_model_name, country, current_timestamp.format("%Y")),
        country,
        province,
        ward: registration_payload.ward.clone(),
        address: registration_payload.address.clone(),
        first_name: registration_payload.first_name.clone(),
        last_name: registration_payload.last_name.clone(),
        email: registration_payload.email.clone(),
        mobile_phone: registration_payload.mobile_phone.clone(),
        should_send_confirmation_email: registration_payload.send_email_confirmation,
        customer_full_name: customer_full_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn req() -> RegisterDeviceRequest {
        RegisterDeviceRequest {
            serial_number: "SN-001".into(),
            province: "HN".into(),
            ward: "Ward 1".into(),
            address: "123 St".into(),
            first_name: "A".into(),
            last_name: "B".into(),
            email: "a@b.com".into(),
            mobile_phone: "0912345678".into(),
            send_email_confirmation: true,
        }
    }

    #[test]
    fn test_success_hn() {
        let result = decide_register_device(
            &req(),
            Uuid::new_v4(),
            "Alice",
            Uuid::new_v4(),
            "MOD-A".into(),
            "ThinkPad".into(),
            chrono::Utc::now(),
        );
        assert!(result.is_ok());
        let effect = result.unwrap();
        assert!(effect.should_send_confirmation_email);
        assert_eq!(effect.country, "Vietnam");
        assert_eq!(effect.province, "HN");
    }

    #[test]
    fn test_success_hcm() {
        let mut request = req();
        request.province = "HCM".into();
        let result = decide_register_device(
            &request,
            Uuid::new_v4(),
            "Alice",
            Uuid::new_v4(),
            "MOD-A".into(),
            "ThinkPad".into(),
            chrono::Utc::now(),
        );
        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.province, "HCM");
    }

    #[test]
    fn test_invalid_province() {
        let mut request = req();
        request.province = "Da Nang".into();
        let result = decide_register_device(
            &request,
            Uuid::new_v4(),
            "Alice",
            Uuid::new_v4(),
            "MOD-A".into(),
            "ThinkPad".into(),
            chrono::Utc::now(),
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert_eq!(msg, "Province must be HN or HCM"),
            _ => panic!("Expected BadRequest"),
        }
    }
}

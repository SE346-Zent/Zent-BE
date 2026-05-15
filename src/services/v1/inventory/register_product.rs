use crate::core::errors::AppError;
use crate::model::requests::inventory::register_product_request::RegisterProductRequest;

pub struct RegisterProductEffect {
    pub product_id: uuid::Uuid,
    pub customer_id: uuid::Uuid,
    pub product_name: String,
    pub model_code: String,
    pub serial_number: String,
    pub should_send_email: bool,
    pub email: String,
    pub customer_name: String,
}

pub fn decide_register_product(
    req: &RegisterProductRequest,
    user_id: uuid::Uuid,
    customer_name: &str,
    model_code_from_catalog: Option<String>,
    model_name_from_catalog: Option<String>,
    existing_product_id: Option<uuid::Uuid>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<RegisterProductEffect, AppError> {
    let model_code = model_code_from_catalog
        .ok_or_else(|| AppError::BadRequest(format!("Serial number '{}' not found in the product catalog", req.serial_number)))?;
    let model_name = model_name_from_catalog
        .ok_or_else(|| AppError::BadRequest(format!("Model name not found for serial '{}'", req.serial_number)))?;

    if let Some(pid) = existing_product_id {
        // Re-registration — returns existing ID, no email
        let effective_country = if req.country.is_empty() { "Vietnam".to_string() } else { req.country.clone() };
        return Ok(RegisterProductEffect {
            product_id: pid,
            customer_id: user_id,
            product_name: format!("{} {} {}", model_name, effective_country, now.format("%Y")),
            model_code,
            serial_number: req.serial_number.clone(),
            should_send_email: false,
            email: req.email.clone(),
            customer_name: customer_name.to_string(),
        });
    }

    let effective_country = if req.country.is_empty() { "Vietnam".to_string() } else { req.country.clone() };
    Ok(RegisterProductEffect {
        product_id: uuid::Uuid::new_v4(),
        customer_id: user_id,
        product_name: format!("{} {} {}", model_name, effective_country, now.format("%Y")),
        model_code,
        serial_number: req.serial_number.clone(),
        should_send_email: req.send_email_confirmation && !req.email.is_empty(),
        email: req.email.clone(),
        customer_name: customer_name.to_string(),
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
        let r = decide_register_product(&req(), u("c0000000-0000-0000-0000-000000000001"), "Alice",
            Some("MOD-A".into()), Some("ThinkPad".into()), None, chrono::Utc::now());
        assert!(r.is_ok());
        let e = r.unwrap();
        assert!(e.should_send_email);
    }

    #[test]
    fn test_unknown_serial() {
        let r = decide_register_product(&req(), u("c0000000-0000-0000-0000-000000000001"), "Alice",
            None, None, None, chrono::Utc::now());
        assert!(r.is_err());
    }

    #[test]
    fn test_duplicate_returns_no_email() {
        let r = decide_register_product(&req(), u("c0000000-0000-0000-0000-000000000001"), "Alice",
            Some("MOD-A".into()), Some("ThinkPad".into()),
            Some(u("10000000-0000-0000-0000-000000000001")), chrono::Utc::now());
        assert!(r.is_ok());
        assert!(!r.unwrap().should_send_email);
    }
}

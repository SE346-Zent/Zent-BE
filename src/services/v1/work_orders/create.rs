use sea_orm::Set;
use crate::{
    core::errors::AppError,
    entities::{work_orders, work_order_state_history},
    model::requests::work_orders::create_work_order_request::CreateWorkOrderRequest,
};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

/// Represents the calculated results and side-effects of a successful work order creation.

#[derive(Debug)]
pub struct CreateWorkOrderEffect {
    /// The database model for the new work order record.
    pub work_order_model: work_orders::ActiveModel,
    /// The database model for the initial state history entry (creation event).
    pub state_history_model: work_order_state_history::ActiveModel,
}

/// Determine the outcome of a work order creation request by validating location policies.
///
/// This pure function ensures that the requested service location is within
/// supported cities (currently HCM and HN), generates a unique work order number,
/// and prepares the database models for the work order and its initial state history.
///
/// # Arguments
/// * `creation_payload` - The request containing work order details (customer info, product, symptom).
/// * `requesting_customer_id` - The unique identifier of the customer creating the work order.
/// * `initial_status_id` - The ID of the default 'Pending' status for new work orders.
/// * `policies` - A map of configuration policies (workday start/end hours).
///
/// # Returns
/// A result containing the `CreateWorkOrderEffect` on success, or an `AppError` for policy violations.

pub fn decide_create_work_order(
    creation_payload: CreateWorkOrderRequest,
    requesting_customer_id: Uuid,
    initial_status_id: i32,
    policies: &HashMap<String, String>,
) -> Result<CreateWorkOrderEffect, AppError> {
    // 1. Location Policy Validation
    if creation_payload.city != "HCM" && creation_payload.city != "HN" {
        tracing::warn!(
            error.message = "UnsupportedCity",
            error.details = "",
            city = %creation_payload.city,
            customer_id = %requesting_customer_id,
            message = "Only HCM and HN are supported at this time"
        );
        return Err(AppError::BadRequest("Only HCM and HN are supported at this time".to_string()));
    }

    // 2. Workday Hours Validation
    let appointment_local = crate::utils::time::to_utc7_time(creation_payload.appointment);

    let workday_start: u32 = match policies.get("workday_start") {
        None => {
            tracing::error!(
                error.message = "MissingWorkdayStartPolicy",
                error.details = "",
                customer_id = %requesting_customer_id,
                message = "Missing workday_start policy"
            );
            return Err(AppError::Internal(anyhow::anyhow!("Missing workday_start policy")));
        }
        Some(val) => match val.parse() {
            Err(_) => {
                tracing::error!(
                    error.message = "InvalidWorkdayStartPolicy",
                    error.details = "",
                    customer_id = %requesting_customer_id,
                    message = "Invalid workday_start policy"
                );
                return Err(AppError::Internal(anyhow::anyhow!("Invalid workday_start policy")));
            }
            Ok(parsed) => parsed,
        }
    };

    let workday_end: u32 = match policies.get("workday_end") {
        None => {
            tracing::error!(
                error.message = "MissingWorkdayEndPolicy",
                error.details = "",
                customer_id = %requesting_customer_id,
                message = "Missing workday_end policy"
            );
            return Err(AppError::Internal(anyhow::anyhow!("Missing workday_end policy")));
        }
        Some(val) => match val.parse() {
            Err(_) => {
                tracing::error!(
                    error.message = "InvalidWorkdayEndPolicy",
                    error.details = "",
                    customer_id = %requesting_customer_id,
                    message = "Invalid workday_end policy"
                );
                return Err(AppError::Internal(anyhow::anyhow!("Invalid workday_end policy")));
            }
            Ok(parsed) => parsed,
        }
    };

    use chrono::Timelike;
    let hour = appointment_local.hour();
    if hour < workday_start || hour >= workday_end {
        tracing::warn!(
            error.message = "AppointmentOutsideWorkdayLimits",
            error.details = "",
            appointment = %creation_payload.appointment,
            hour = %hour,
            workday_start = %workday_start,
            workday_end = %workday_end,
            customer_id = %requesting_customer_id,
            message = "Appointment hour is outside workday limits"
        );
        return Err(AppError::BadRequest(format!(
            "Appointment hour {:02}:{:02} is outside workday limits ({:02}:00 - {:02}:00)",
            hour,
            appointment_local.minute(),
            workday_start,
            workday_end
        )));
    }

    // 3. ID and Number Generation
    let current_timestamp = Utc::now();
    let work_order_id = Uuid::new_v4();
    let work_order_number = format!("WO-{}", &work_order_id.to_string()[..6].to_uppercase());

    
    let work_order_active_model = work_orders::ActiveModel {
        id: Set(work_order_id),
        work_order_status_id: Set(initial_status_id),
        customer_id: Set(requesting_customer_id),
        product_id: Set(creation_payload.product_id),
        reference_ticket_id: Set(creation_payload.reference_ticket_id),
        work_order_symptom_id: Set(creation_payload.work_order_symptom_id),
        description: Set(creation_payload.description),
        first_name: Set(creation_payload.first_name),
        last_name: Set(creation_payload.last_name),
        email: Set(creation_payload.email),
        phone_number: Set(creation_payload.phone_number),
        country: Set(creation_payload.country),
        province: Set(creation_payload.province),
        city: Set(creation_payload.city),
        address: Set(creation_payload.address),
        building: Set(creation_payload.building),
        appointment: Set(creation_payload.appointment),
        work_order_number: Set(work_order_number),
        created_at: Set(current_timestamp),
        updated_at: Set(current_timestamp),
        ..Default::default()
    };

    let state_history_active_model = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order_id),
        from_status_id: Set(None), // Initial creation — no previous status
        to_status_id: Set(initial_status_id),
        changed_by_id: Set(requesting_customer_id),
        changed_at: Set(current_timestamp),
    };

    tracing::info!(
        reason = "CreateWorkOrderSuccess",
        customer_id = %requesting_customer_id,
        work_order_id = %work_order_id,
        message = "Successfully decided to create work order"
    );

    Ok(CreateWorkOrderEffect { work_order_model: work_order_active_model, state_history_model: state_history_active_model })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn get_mock_policies() -> HashMap<String, String> {
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("workday_end".to_string(), "17".to_string());
        policies
    }

    #[test]
    fn test_decide_create_work_order_success() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;
        let policies = get_mock_policies();

        let req = CreateWorkOrderRequest {
            product_id: Uuid::new_v4(),
            work_order_symptom_id: 1,
            reference_ticket_id: None,
            description: "Issue".to_string(),
            appointment: Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap(), // 10:00 AM GMT+7
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: None,
            phone_number: None,
            country: "VN".to_string(),
            province: "HCM".to_string(),
            city: "HCM".to_string(),
            address: "123 Street".to_string(),
            building: None,
        };

        let result = decide_create_work_order(req, customer_id, pending_status_id, &policies);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order_model.work_order_status_id, Set(pending_status_id));
        assert_eq!(effect.work_order_model.city, Set("HCM".to_string()));
        assert_eq!(effect.state_history_model.to_status_id, Set(pending_status_id));
        assert_eq!(effect.state_history_model.from_status_id, Set(None));
    }

    #[test]
    fn test_decide_create_work_order_invalid_location() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;
        let policies = get_mock_policies();

        let req = CreateWorkOrderRequest {
            product_id: Uuid::new_v4(),
            work_order_symptom_id: 1,
            reference_ticket_id: None,
            description: "Issue".to_string(),
            appointment: Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap(), // 10:00 AM GMT+7
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: None,
            phone_number: None,
            country: "VN".to_string(),
            province: "Binh Duong".to_string(),
            city: "Binh Duong".to_string(), // Invalid
            address: "123 Street".to_string(),
            building: None,
        };

        let result = decide_create_work_order(req, customer_id, pending_status_id, &policies);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert_eq!(msg, "Only HCM and HN are supported at this time"),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_decide_create_work_order_invalid_workhours() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;
        let policies = get_mock_policies();

        let req = CreateWorkOrderRequest {
            product_id: Uuid::new_v4(),
            work_order_symptom_id: 1,
            reference_ticket_id: None,
            description: "Issue".to_string(),
            appointment: Utc.with_ymd_and_hms(2026, 1, 1, 15, 0, 0).unwrap(), // 22:00 GMT+7
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: None,
            phone_number: None,
            country: "VN".to_string(),
            province: "HCM".to_string(),
            city: "HCM".to_string(),
            address: "123 Street".to_string(),
            building: None,
        };

        let result = decide_create_work_order(req, customer_id, pending_status_id, &policies);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert!(msg.contains("outside workday limits")),
            _ => panic!("Expected BadRequest"),
        }
    }
}


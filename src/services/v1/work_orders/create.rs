use sea_orm::Set;
use crate::{
    core::errors::AppError,
    entities::{work_orders, work_order_state_history},
    model::requests::work_orders::create_work_order_request::CreateWorkOrderRequest,
};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub struct CreateWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_create_work_order(
    req: CreateWorkOrderRequest,
    customer_id: Uuid,
    pending_status_id: i32,
) -> Result<CreateWorkOrderEffect, AppError> {
    // 1. Location Policy Validation
    if req.city != "HCM" && req.city != "HN" {
        return Err(AppError::BadRequest("Only HCM and HN are supported at this time".to_string()));
    }

    // 2. ID and Number Generation
    let now = Utc::now();
    let wo_id = Uuid::new_v4();
    let work_order_number = format!("WO-{}", &wo_id.to_string()[..6].to_uppercase());

    
    let work_order = work_orders::ActiveModel {
        id: Set(wo_id),
        work_order_status_id: Set(pending_status_id),
        customer_id: Set(customer_id),
        product_id: Set(req.product_id),
        reference_ticket_id: Set(req.reference_ticket_id),
        work_order_symptom_id: Set(req.work_order_symptom_id),
        description: Set(req.description),
        first_name: Set(req.first_name),
        last_name: Set(req.last_name),
        email: Set(req.email),
        phone_number: Set(req.phone_number),
        country: Set(req.country),
        province: Set(req.province),
        city: Set(req.city),
        address: Set(req.address),
        building: Set(req.building),
        appointment: Set(req.appointment),
        work_order_number: Set(work_order_number),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(wo_id),
        from_status_id: Set(None), // Initial creation — no previous status
        to_status_id: Set(pending_status_id),
        changed_by_id: Set(customer_id),
        changed_at: Set(now),
    };

    Ok(CreateWorkOrderEffect { work_order, state_history })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decide_create_work_order_success() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;

        let req = CreateWorkOrderRequest {
            product_id: Uuid::new_v4(),
            work_order_symptom_id: 1,
            reference_ticket_id: None,
            description: "Issue".to_string(),
            appointment: Utc::now(),
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

        let result = decide_create_work_order(req, customer_id, pending_status_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order.work_order_status_id, Set(pending_status_id));
        assert_eq!(effect.work_order.city, Set("HCM".to_string()));
        assert_eq!(effect.state_history.to_status_id, Set(pending_status_id));
        assert_eq!(effect.state_history.from_status_id, Set(None));
    }

    #[test]
    fn test_decide_create_work_order_invalid_location() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;

        let req = CreateWorkOrderRequest {
            product_id: Uuid::new_v4(),
            work_order_symptom_id: 1,
            reference_ticket_id: None,
            description: "Issue".to_string(),
            appointment: Utc::now(),
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

        let result = decide_create_work_order(req, customer_id, pending_status_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert_eq!(msg, "Only HCM and HN are supported at this time"),
            _ => panic!("Expected BadRequest"),
        }
    }
}


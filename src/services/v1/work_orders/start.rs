use std::collections::HashMap;
use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_state_history, work_orders},
    model::requests::work_orders::start_request::StartWorkOrderRequest,
};

#[derive(Debug)]
pub struct StartWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub async fn decide_start(
    payload: StartWorkOrderRequest,
    work_order: work_orders::Model,
    technician_id: Uuid,
    in_progress_status_id: i32,
    policies: &HashMap<String, String>,
    target_lat: f64,
    target_lng: f64,
) -> Result<StartWorkOrderEffect, AppError> {
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // Geofencing Check
    let radius: f64 = policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0);

    let is_verified = crate::utils::geo::is_within_geofence(
        payload.latitude,
        payload.longitude,
        target_lat,
        target_lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    let now = Utc::now();
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(in_progress_status_id);
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(in_progress_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(now),
    };

    Ok(StartWorkOrderEffect {
        work_order: active_wo,
        state_history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order(tech_id: Uuid) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: 2, // Assigned
            customer_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            reference_ticket_id: None,
            work_order_symptom_id: 1,
            description: "".to_string(),
            first_name: "".to_string(),
            last_name: "".to_string(),
            email: None,
            phone_number: None,
            country: "Vietnam".to_string(),
            province: "Ho Chi Minh City".to_string(),
            city: "Ho Chi Minh City".to_string(),
            address: "123 Le Loi".to_string(),
            building: None,
            appointment: Utc::now(),
            admin_id: None,
            technician_id: Some(tech_id),
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            customer_complaint: None,
            customer_complaint_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    #[tokio::test]
    async fn test_decide_start_success() {
        let tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);
        let in_progress_status_id = 3;

        let req = StartWorkOrderRequest {
            latitude: 10.774502,
            longitude: 106.702958,
        };

        let mut policies = HashMap::new();
        policies.insert("geofencing_radius".to_string(), "2000".to_string());

        // Use same coordinates as target to ensure it is within geofence
        let result = decide_start(req, wo, tech_id, in_progress_status_id, &policies, 10.7769, 106.7009).await;
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order.work_order_status_id, Set(in_progress_status_id));
        assert_eq!(effect.state_history.to_status_id, Set(in_progress_status_id));
    }

    #[tokio::test]
    async fn test_decide_start_geofence_violation() {
        let tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);
        let in_progress_status_id = 3;

        let req = StartWorkOrderRequest {
            latitude: 40.712776,
            longitude: -74.005974,
        };

        let mut policies = HashMap::new();
        policies.insert("geofencing_radius".to_string(), "500".to_string());

        // Pass coordinates in HCM, which is far from New York
        let result = decide_start(req, wo, tech_id, in_progress_status_id, &policies, 10.7769, 106.7009).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(msg) => assert!(msg.contains("Geofencing violation")),
            _ => panic!("Expected Forbidden"),
        }
    }

    #[tokio::test]
    async fn test_decide_start_forbidden_technician() {
        let tech_id = Uuid::new_v4();
        let wrong_tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);
        let in_progress_status_id = 3;

        let req = StartWorkOrderRequest { latitude: 0.0, longitude: 0.0 };
        let policies = HashMap::new();

        let result = decide_start(req, wo, wrong_tech_id, in_progress_status_id, &policies, 0.0, 0.0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(msg) => assert_eq!(msg, "You are not assigned to this work order"),
            _ => panic!("Expected Forbidden"),
        }
    }
}


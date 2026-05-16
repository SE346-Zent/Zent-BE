use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_pause_actions, work_order_state_history, work_orders},
};

#[derive(Debug)]
pub struct PauseWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub audit: work_order_pause_actions::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

/// Pure logic: decide whether a technician can pause a work order.
///
/// Rules:
/// - Technician must be assigned to the work order
/// - Work order must be in "InProg" status (can only pause work that's in progress)
/// - Reason is required (validated at the request layer)
pub fn decide_pause_work_order(
    work_order: work_orders::Model,
    reason: String,
    explanation: Option<String>,
    in_prog_status_id: i32,
    paused_status_id: i32,
    technician_id: Uuid,
) -> Result<PauseWorkOrderEffect, AppError> {
    // Only the assigned technician can pause
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden(
            "You are not assigned to this work order".into(),
        ));
    }

    // Can only pause when in progress
    if work_order.work_order_status_id != in_prog_status_id {
        return Err(AppError::BadRequest(
            "Can only pause a work order that is In Progress".into(),
        ));
    }

    let now = Utc::now();

    // Transition work order to Paused
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(paused_status_id);
    active_wo.updated_at = Set(now);

    // Audit row
    let audit = work_order_pause_actions::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        paused_by_id: Set(technician_id),
        reason: Set(reason),
        explanation: Set(explanation),
        created_at: Set(now),
    };

    // State history
    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(paused_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(now),
    };

    Ok(PauseWorkOrderEffect {
        work_order: active_wo,
        audit,
        state_history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order(status_id: i32, tech_id: Uuid) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: status_id,
            customer_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            reference_ticket_id: None,
            work_order_symptom_id: 1,
            description: "".to_string(),
            first_name: "".to_string(),
            last_name: "".to_string(),
            email: None,
            phone_number: None,
            country: "".to_string(),
            province: "".to_string(),
            city: "".to_string(),
            address: "".to_string(),
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
        }
    }

    #[test]
    fn test_pause_success() {
        let tech_id = Uuid::new_v4();
        let wo = dummy_work_order(3, tech_id); // InProg
        let result = decide_pause_work_order(
            wo,
            "Waiting for replacement part".into(),
            Some("The LCD panel is out of stock".into()),
            3,  // in_prog_status_id
            7,  // paused_status_id
            tech_id,
        );
        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.work_order.work_order_status_id, Set(7));
        assert_eq!(effect.audit.reason, Set("Waiting for replacement part".into()));
        assert_eq!(effect.state_history.to_status_id, Set(7));
    }

    #[test]
    fn test_pause_wrong_technician() {
        let tech_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        let wo = dummy_work_order(3, tech_id);
        let result = decide_pause_work_order(
            wo,
            "Reason here".into(),
            None,
            3, 7,
            other_id,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(_) => {}
            _ => panic!("Expected Forbidden"),
        }
    }

    #[test]
    fn test_pause_wrong_status() {
        let tech_id = Uuid::new_v4();
        let wo = dummy_work_order(2, tech_id); // Assigned, not InProg
        let result = decide_pause_work_order(
            wo,
            "Reason here".into(),
            None,
            3, 7,
            tech_id,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("In Progress"));
            }
            _ => panic!("Expected BadRequest"),
        }
    }
}

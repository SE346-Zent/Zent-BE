use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_reject_forms, work_order_state_history, work_orders},
};

/// Represents the calculated results and side-effects of an administrator approving a refusal.

#[derive(Debug)]
pub struct ApproveRefusalEffect {
    /// The database model for the updated work order (transitioned to 'Refused' status).
    pub work_order_model: work_orders::ActiveModel,
    /// The database model for the updated rejection form (marked as approved).
    pub reject_form_model: work_order_reject_forms::ActiveModel,
    /// The database model for the state history entry recording the refusal approval.
    pub state_history_model: work_order_state_history::ActiveModel,
}

/// Determine the outcome of approving a technician's refusal of a work order.
///
/// This function verifies that the work order and rejection form are correctly linked,
/// marks the form as approved by the administrator, and transitions the work order
/// to a permanent 'Refused' status.

/// Admin approves the technician's refusal.
/// This means the admin accepts the reason and permanently refuses the work order.
pub fn decide_approve_refusal(
    work_order: work_orders::Model,
    reject_form: work_order_reject_forms::Model,
    admin_id: Uuid,
    target_refused_status_id: i32,
) -> Result<ApproveRefusalEffect, AppError> {
    if work_order.reject_form_id != Some(reject_form.id) {
        return Err(AppError::BadRequest("Rejection form does not match this work order".to_string()));
    }

    let current_timestamp = Utc::now();

    // 1. Mark form as approved
    let mut reject_form_active_model: work_order_reject_forms::ActiveModel = reject_form.into();
    reject_form_active_model.approved = Set(true);
    reject_form_active_model.approver_id = Set(Some(admin_id));
    reject_form_active_model.updated_at = Set(Some(current_timestamp));

    // 2. Terminate work order
    let mut work_order_active_model: work_orders::ActiveModel = work_order.clone().into();
    work_order_active_model.work_order_status_id = Set(target_refused_status_id);
    work_order_active_model.updated_at = Set(current_timestamp);

    let state_history_active_model = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(target_refused_status_id),
        changed_by_id: Set(admin_id),
        changed_at: Set(current_timestamp),
    };

    Ok(ApproveRefusalEffect {
        work_order_model: work_order_active_model,
        reject_form_model: reject_form_active_model,
        state_history_model: state_history_active_model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order() -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: 6, // Refused
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
            ward: "".to_string(),
            address: "".to_string(),
            building: None,
            appointment: Utc::now(),
            admin_id: None,
            technician_id: Some(Uuid::new_v4()),
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    fn dummy_reject_form() -> work_order_reject_forms::Model {
        work_order_reject_forms::Model {
            id: Uuid::new_v4(),
            approver_id: None,
            approved: false,
            reason: "".to_string(),
            explanation: "".to_string(),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }

    #[test]
    fn test_decide_approve_refusal_success() {
        let mut work_order = dummy_work_order();
        let reject_form = dummy_reject_form();
        work_order.reject_form_id = Some(reject_form.id);
        
        let admin_id = Uuid::new_v4();
        let target_refused_status_id = 99;

        let result = decide_approve_refusal(work_order, reject_form.clone(), admin_id, target_refused_status_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order_model.work_order_status_id, Set(target_refused_status_id));
        assert_eq!(effect.reject_form_model.approved, Set(true));
        assert_eq!(effect.reject_form_model.approver_id, Set(Some(admin_id)));
        assert_eq!(effect.state_history_model.to_status_id, Set(target_refused_status_id));
    }

    #[test]
    fn test_decide_approve_refusal_mismatch() {
        let work_order = dummy_work_order();
        let reject_form = dummy_reject_form();
        
        let admin_id = Uuid::new_v4();
        let target_refused_status_id = 99;

        let result = decide_approve_refusal(work_order, reject_form, admin_id, target_refused_status_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert_eq!(msg, "Rejection form does not match this work order"),
            _ => panic!("Expected BadRequest"),
        }
    }
}


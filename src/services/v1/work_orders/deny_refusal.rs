use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_reject_forms, work_order_state_history, work_orders},
};

/// Represents the calculated results and side-effects of an administrator denying a refusal.

#[derive(Debug)]
pub struct DenyRefuseEffect {
    /// The database model for the updated work order (reset to 'Pending' and unassigned).
    pub work_order_model: work_orders::ActiveModel,
    /// The database model for the updated rejection form (marked as denied/unapproved).
    pub reject_form_model: work_order_reject_forms::ActiveModel,
    /// The database model for the state history entry recording the refusal denial.
    pub state_history_model: work_order_state_history::ActiveModel,
}

/// Determine the outcome of denying a technician's refusal of a work order.
///
/// This function verifies the linkage between the work order and rejection form,
/// marks the form as denied by the administrator, and resets the work order
/// to 'Pending' status with no technician assigned.

/// Admin denies the technician's refusal.
/// This means the admin disagrees. The work order is reset to 'Pending'
/// and the technician is removed so it can be manually reassigned.
pub fn decide_deny_refusal(
    work_order: work_orders::Model,
    reject_form: work_order_reject_forms::Model,
    admin_id: Uuid,
    target_pending_status_id: i32,
) -> Result<DenyRefuseEffect, AppError> {
    if work_order.reject_form_id != Some(reject_form.id) {
        return Err(AppError::BadRequest("Work order does not match this rejection form".to_string()));
    }

    let current_timestamp = Utc::now();

    // 1. Mark form as NOT approved (Denied)
    let mut reject_form_active_model: work_order_reject_forms::ActiveModel = reject_form.into();
    reject_form_active_model.approved = Set(false);
    reject_form_active_model.approver_id = Set(Some(admin_id));
    reject_form_active_model.updated_at = Set(Some(current_timestamp));

    // 2. Reset Work Order to 'Pending' and clear technician
    let mut work_order_active_model: work_orders::ActiveModel = work_order.clone().into();
    work_order_active_model.work_order_status_id = Set(target_pending_status_id);
    work_order_active_model.technician_id = Set(None);
    work_order_active_model.reject_form_id = Set(None); // Detach form from active flow
    work_order_active_model.updated_at = Set(current_timestamp);

    let state_history_active_model = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(target_pending_status_id),
        changed_by_id: Set(admin_id),
        changed_at: Set(current_timestamp),
    };

    Ok(DenyRefuseEffect {
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
            city: "".to_string(),
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
    fn test_decide_deny_refusal_success() {
        let mut work_order = dummy_work_order();
        let reject_form = dummy_reject_form();
        work_order.reject_form_id = Some(reject_form.id);
        
        let admin_id = Uuid::new_v4();
        let target_pending_status_id = 1;

        let result = decide_deny_refusal(work_order, reject_form.clone(), admin_id, target_pending_status_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order_model.work_order_status_id, Set(target_pending_status_id));
        assert_eq!(effect.work_order_model.technician_id, Set(None));
        assert_eq!(effect.work_order_model.reject_form_id, Set(None));
        assert_eq!(effect.reject_form_model.approved, Set(false));
        assert_eq!(effect.reject_form_model.approver_id, Set(Some(admin_id)));
        assert_eq!(effect.state_history_model.to_status_id, Set(target_pending_status_id));
    }

    #[test]
    fn test_decide_deny_refusal_mismatch() {
        let work_order = dummy_work_order();
        let reject_form = dummy_reject_form();
        
        let admin_id = Uuid::new_v4();
        let target_pending_status_id = 1;

        let result = decide_deny_refusal(work_order, reject_form, admin_id, target_pending_status_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert_eq!(msg, "Work order does not match this rejection form"),
            _ => panic!("Expected BadRequest"),
        }
    }
}


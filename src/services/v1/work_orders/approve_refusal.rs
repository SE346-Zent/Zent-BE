use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_reject_forms, work_order_state_history, work_orders},
};

#[derive(Debug)]
pub struct ApproveRefusalEffect {
    pub work_order: work_orders::ActiveModel,
    pub reject_form: work_order_reject_forms::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

/// Admin approves the technician's refusal.
/// This means the admin accepts the reason and permanently refuses the work order.
pub fn decide_approve_refusal(
    work_order: work_orders::Model,
    reject_form: work_order_reject_forms::Model,
    admin_id: Uuid,
    refused_status_id: i32,
) -> Result<ApproveRefusalEffect, AppError> {
    if work_order.reject_form_id != Some(reject_form.id) {
        return Err(AppError::BadRequest("Work order does not match this rejection form".to_string()));
    }

    let now = Utc::now();

    // 1. Mark form as approved
    let mut active_form: work_order_reject_forms::ActiveModel = reject_form.into();
    active_form.approved = Set(true);
    active_form.approver_id = Set(Some(admin_id));
    active_form.updated_at = Set(Some(now));

    // 2. Terminate work order
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(refused_status_id);
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(refused_status_id),
        changed_by_id: Set(admin_id),
        changed_at: Set(now),
    };

    Ok(ApproveRefusalEffect {
        work_order: active_wo,
        reject_form: active_form,
        state_history,
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
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
        let mut wo = dummy_work_order();
        let rf = dummy_reject_form();
        wo.reject_form_id = Some(rf.id);
        
        let admin_id = Uuid::new_v4();
        let rejected_status_id = 99;

        let result = decide_approve_refusal(wo, rf.clone(), admin_id, rejected_status_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order.work_order_status_id, Set(rejected_status_id));
        assert_eq!(effect.reject_form.approved, Set(true));
        assert_eq!(effect.reject_form.approver_id, Set(Some(admin_id)));
        assert_eq!(effect.state_history.to_status_id, Set(rejected_status_id));
    }

    #[test]
    fn test_decide_approve_refusal_mismatch() {
        let wo = dummy_work_order();
        let rf = dummy_reject_form();
        
        let admin_id = Uuid::new_v4();
        let rejected_status_id = 99;

        let result = decide_approve_refusal(wo, rf, admin_id, rejected_status_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert_eq!(msg, "Work order does not match this rejection form"),
            _ => panic!("Expected BadRequest"),
        }
    }
}


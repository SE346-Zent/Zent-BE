use crate::{
    core::errors::AppError,
    entities::{work_order_state_history, work_orders},
};
use uuid::Uuid;

#[derive(Debug)]
pub struct CancelWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_cancel(
    work_order: work_orders::Model,
    customer_id: Uuid,
    cancelled_status_id: i32,
    pending_status_id: i32,
) -> Result<CancelWorkOrderEffect, AppError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order(customer_id: Uuid, status_id: i32) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: status_id,
            customer_id,
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
            technician_id: None,
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_success() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;
        let cancelled_status_id = 99;
        let wo = dummy_work_order(customer_id, pending_status_id);

        let result = decide_cancel(wo, customer_id, cancelled_status_id, pending_status_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(
            effect.work_order.work_order_status_id,
            Set(cancelled_status_id)
        );
        assert_eq!(effect.state_history.to_status_id, Set(cancelled_status_id));
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_wrong_customer() {
        let customer_id = Uuid::new_v4();
        let wrong_customer_id = Uuid::new_v4();
        let pending_status_id = 1;
        let cancelled_status_id = 99;
        let wo = dummy_work_order(customer_id, pending_status_id);

        let result = decide_cancel(
            wo,
            wrong_customer_id,
            cancelled_status_id,
            pending_status_id,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(_) => {}
            _ => panic!("Expected Forbidden"),
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_not_pending() {
        let customer_id = Uuid::new_v4();
        let in_progress_status_id = 3;
        let pending_status_id = 1;
        let cancelled_status_id = 99;
        let wo = dummy_work_order(customer_id, in_progress_status_id);

        let result = decide_cancel(wo, customer_id, cancelled_status_id, pending_status_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(_) => {}
            _ => panic!("Expected BadRequest"),
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_already_cancelled() {
        let customer_id = Uuid::new_v4();
        let pending_status_id = 1;
        let cancelled_status_id = 99;
        let wo = dummy_work_order(customer_id, cancelled_status_id);

        let result = decide_cancel(wo, customer_id, cancelled_status_id, pending_status_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Conflict(_) => {}
            _ => panic!("Expected Conflict"),
        }
    }
}

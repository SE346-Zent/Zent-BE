use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use crate::entities::{work_orders, work_order_reject_forms, images, work_order_reject_form_image_links, work_order_state_history};
use crate::model::requests::work_orders::refuse_request::RefuseWorkOrderRequest;
use crate::core::errors::AppError;

/// Represents the calculated results and side-effects of a technician refusing a work order.

#[derive(Debug)]
pub struct RefuseEffect {
    /// The database model for the updated work order (transitioned to review status).
    pub work_order_model: work_orders::ActiveModel,
    /// The database model for the refusal/rejection form details.
    pub reject_form_model: work_order_reject_forms::ActiveModel,
    /// Database models for any evidence images provided with the refusal.
    pub image_models: Vec<images::ActiveModel>,
    /// Database models linking the images to the rejection form.
    pub image_link_models: Vec<work_order_reject_form_image_links::ActiveModel>,
    /// The database model for the state history entry recording the refusal event.
    pub state_history_model: work_order_state_history::ActiveModel,
}

/// Determine the outcome of a work order refusal request by a technician.
///
/// This function validates that the technician is assigned to the work order,
/// creates a formal rejection form with reasons and evidence images, and
/// transitions the work order to a review status for administrator approval.

pub fn decide_refuse_work_order(
    refusal_payload: RefuseWorkOrderRequest,
    work_order: work_orders::Model,
    target_refusal_status_id: i32,
    technician_id: Uuid,
) -> Result<RefuseEffect, AppError> {
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let reject_form_id = Uuid::new_v4();
    let current_timestamp = Utc::now();

    let mut new_image_models = Vec::new();
    let mut new_image_link_models = Vec::new();

    for url in refusal_payload.evidence_image_urls {
        let image_id = Uuid::new_v4();
        new_image_models.push(images::ActiveModel {
            id: Set(image_id),
            object_name: Set(url),
            created_at: Set(current_timestamp),
            updated_at: Set(current_timestamp),
            ..Default::default()
        });

        new_image_link_models.push(work_order_reject_form_image_links::ActiveModel {
            image_id: Set(image_id),
            work_order_reject_form_id: Set(reject_form_id),
        });
    }

    let reject_form_active_model = work_order_reject_forms::ActiveModel {
        id: Set(reject_form_id),
        approver_id: Set(None), // Will be filled when an admin reviews it
        approved: Set(false),
        reason: Set(refusal_payload.reason),
        explanation: Set(refusal_payload.explanation),
        created_at: Set(Some(current_timestamp)),
        updated_at: Set(Some(current_timestamp)),
    };

    let old_status_id = work_order.work_order_status_id;

    let mut work_order_active_model: work_orders::ActiveModel = work_order.into();
    work_order_active_model.work_order_status_id = Set(target_refusal_status_id);
    work_order_active_model.reject_form_id = Set(Some(reject_form_id));
    work_order_active_model.updated_at = Set(current_timestamp);

    let state_history_active_model = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order_active_model.id.clone().unwrap()),
        from_status_id: Set(Some(old_status_id)),
        to_status_id: Set(target_refusal_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(current_timestamp),
    };

    Ok(RefuseEffect {
        work_order_model: work_order_active_model,
        reject_form_model: reject_form_active_model,
        image_models: new_image_models,
        image_link_models: new_image_link_models,
        state_history_model: state_history_active_model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order(tech_id: Uuid) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: 3, // In Progress
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    #[test]
    fn test_decide_refuse_work_order_success() {
        let tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);
        let refuse_in_review_status_id = 5;

        let req = RefuseWorkOrderRequest {
            reason: "Out of scope".to_string(),
            explanation: "Requires specialized equipment".to_string(),
            evidence_image_urls: vec!["http://example.com/img1.png".to_string()],
        };

        let result = decide_refuse_work_order(req, wo, refuse_in_review_status_id, tech_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order_model.work_order_status_id, Set(refuse_in_review_status_id));
        assert!(effect.work_order_model.reject_form_id.is_set());
        assert_eq!(effect.reject_form_model.reason, Set("Out of scope".to_string()));
        assert_eq!(effect.reject_form_model.approved, Set(false));
        
        assert_eq!(effect.image_models.len(), 1);
        assert_eq!(effect.image_models[0].object_name, Set("http://example.com/img1.png".to_string()));
        
        assert_eq!(effect.image_link_models.len(), 1);
        
        assert_eq!(effect.state_history_model.to_status_id, Set(refuse_in_review_status_id));
    }

    #[test]
    fn test_decide_refuse_work_order_forbidden() {
        let tech_id = Uuid::new_v4();
        let wrong_tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);
        let refuse_in_review_status_id = 5;

        let req = RefuseWorkOrderRequest {
            reason: "Out of scope".to_string(),
            explanation: "".to_string(),
            evidence_image_urls: vec![],
        };

        let result = decide_refuse_work_order(req, wo, refuse_in_review_status_id, wrong_tech_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(_) => {},
            _ => panic!("Expected Forbidden"),
        }
    }
}


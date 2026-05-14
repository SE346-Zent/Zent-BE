use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use crate::entities::{work_orders, work_order_reject_forms, images, work_order_reject_form_image_links, work_order_state_history};
use crate::model::requests::work_orders::refuse_request::RefuseWorkOrderRequest;
use crate::core::errors::AppError;

#[derive(Debug)]
pub struct RefuseEffect {
    pub work_order: work_orders::ActiveModel,
    pub reject_form: work_order_reject_forms::ActiveModel,
    pub images: Vec<images::ActiveModel>,
    pub image_links: Vec<work_order_reject_form_image_links::ActiveModel>,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_refuse_work_order(
    payload: RefuseWorkOrderRequest,
    work_order: work_orders::Model,
    refuse_in_review_status_id: i32,
    technician_id: Uuid,
) -> Result<RefuseEffect, AppError> {
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let reject_form_id = Uuid::new_v4();
    let now = Utc::now();

    let mut images_to_insert = Vec::new();
    let mut image_links_to_insert = Vec::new();

    for url in payload.evidence_image_urls {
        let image_id = Uuid::new_v4();
        images_to_insert.push(images::ActiveModel {
            id: Set(image_id),
            object_name: Set(url),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });

        image_links_to_insert.push(work_order_reject_form_image_links::ActiveModel {
            image_id: Set(image_id),
            work_order_reject_form_id: Set(reject_form_id),
        });
    }

    let reject_form = work_order_reject_forms::ActiveModel {
        id: Set(reject_form_id),
        approver_id: Set(None), // Will be filled when an admin reviews it
        approved: Set(false),
        reason: Set(payload.reason),
        explanation: Set(payload.explanation),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
    };

    let old_status_id = work_order.work_order_status_id;

    let mut work_order_active: work_orders::ActiveModel = work_order.into();
    work_order_active.work_order_status_id = Set(refuse_in_review_status_id);
    work_order_active.reject_form_id = Set(Some(reject_form_id));
    work_order_active.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order_active.id.clone().unwrap()),
        from_status_id: Set(Some(old_status_id)),
        to_status_id: Set(refuse_in_review_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(now),
    };

    Ok(RefuseEffect {
        work_order: work_order_active,
        reject_form,
        images: images_to_insert,
        image_links: image_links_to_insert,
        state_history,
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

        assert_eq!(effect.work_order.work_order_status_id, Set(refuse_in_review_status_id));
        assert!(effect.work_order.reject_form_id.is_set());
        assert_eq!(effect.reject_form.reason, Set("Out of scope".to_string()));
        assert_eq!(effect.reject_form.approved, Set(false));
        
        assert_eq!(effect.images.len(), 1);
        assert_eq!(effect.images[0].object_name, Set("http://example.com/img1.png".to_string()));
        
        assert_eq!(effect.image_links.len(), 1);
        
        assert_eq!(effect.state_history.to_status_id, Set(refuse_in_review_status_id));
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


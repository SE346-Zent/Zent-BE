use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use crate::entities::{work_orders, work_order_reject_forms, images, work_order_reject_form_image_links, work_order_state_history};
use crate::model::requests::work_orders::refuse_request::RefuseWorkOrderRequest;
use crate::core::errors::AppError;

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

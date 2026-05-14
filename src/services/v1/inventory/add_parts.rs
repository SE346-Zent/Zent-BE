use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{new_part_forms, work_orders, images, new_part_form_image_links},
    model::requests::inventory::add_parts_request::AddPartsRequest,
};

#[derive(Debug)]
pub struct AddPartsEffect {
    pub new_part_form: new_part_forms::ActiveModel,
    pub images: Vec<images::ActiveModel>,
    pub image_links: Vec<new_part_form_image_links::ActiveModel>,
}

pub fn decide_add_parts(
    payload: AddPartsRequest,
    work_order: work_orders::Model,
    technician_id: Uuid,
) -> Result<AddPartsEffect, AppError> {
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let now = Utc::now();
    let form_id = Uuid::new_v4();

    let new_part_form = new_part_forms::ActiveModel {
        id: Set(form_id),
        part_number: Set(payload.part_number),
        part_types_id: Set(payload.part_types_id),
        model_code: Set(payload.model_code),
        serial_number: Set(payload.serial_number),
        description: Set(payload.description),
        work_order_id: Set(work_order.id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    // Create image + link records for each photo filename
    let mut images_to_insert = Vec::new();
    let mut image_links_to_insert = Vec::new();

    for object_name in payload.photos {
        let image_id = Uuid::new_v4();
        images_to_insert.push(images::ActiveModel {
            id: Set(image_id),
            object_name: Set(object_name),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        });

        image_links_to_insert.push(new_part_form_image_links::ActiveModel {
            image_id: Set(image_id),
            new_part_form_id: Set(form_id),
        });
    }

    Ok(AddPartsEffect {
        new_part_form,
        images: images_to_insert,
        image_links: image_links_to_insert,
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
    fn test_decide_add_parts_success() {
        let tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);

        let req = AddPartsRequest {
            part_number: "PN-123".to_string(),
            part_types_id: 1,
            model_code: Some("MC-123".to_string()),
            serial_number: "SN-123".to_string(),
            description: Some("desc".to_string()),
            photos: vec!["img.png".to_string()],
        };

        let result = decide_add_parts(req, wo.clone(), tech_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.new_part_form.part_number, Set("PN-123".to_string()));
        assert_eq!(effect.new_part_form.work_order_id, Set(wo.id));
        
        assert_eq!(effect.images.len(), 1);
        assert_eq!(effect.images[0].object_name, Set("img.png".to_string()));
        assert_eq!(effect.image_links.len(), 1);
    }

    #[test]
    fn test_decide_add_parts_forbidden() {
        let tech_id = Uuid::new_v4();
        let wrong_tech_id = Uuid::new_v4();
        let wo = dummy_work_order(tech_id);

        let req = AddPartsRequest {
            part_number: "PN-123".to_string(),
            part_types_id: 1,
            model_code: None,
            serial_number: "SN-123".to_string(),
            description: None,
            photos: vec![],
        };

        let result = decide_add_parts(req, wo, wrong_tech_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(_) => {},
            _ => panic!("Expected Forbidden"),
        }
    }
}


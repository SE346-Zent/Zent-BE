use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{new_part_forms, work_orders, images, new_part_form_image_links},
    model::requests::inventory::add_parts_request::AddPartsRequest,
};

/// Represents the calculated results and side-effects of a successful part addition request.
///
/// This structure prepares the database models for the new part form, the
/// associated images, and the links between them, which will later be persisted
/// in a single transaction.
#[derive(Debug)]
pub struct AddPartsEffect {
    /// The database model for the part registration form.
    pub part_form_model: new_part_forms::ActiveModel,
    /// A list of database models for the uploaded part photos.
    pub image_models: Vec<images::ActiveModel>,
    /// A list of database models linking the part form to its photos.
    pub image_link_models: Vec<new_part_form_image_links::ActiveModel>,
}

/// Determine the outcome of a part addition request by validating technician assignment and preparing data.
///
/// This pure function ensures that only the technician currently assigned to the
/// work order can add parts, and prepares the models for the registration form
/// and its associated images.
///
/// # Arguments
/// * `add_parts_payload` - The request containing part details and photo filenames.
/// * `work_order_record` - The database model representing the associated work order.
/// * `requesting_technician_id` - The unique identifier of the technician attempting the addition.
///
/// # Returns
/// A result containing the `AddPartsEffect` on success, or a `Forbidden` error if the technician is not assigned.
pub fn decide_add_parts(
    add_parts_payload: AddPartsRequest,
    work_order_record: work_orders::Model,
    requesting_technician_id: Uuid,
) -> Result<AddPartsEffect, AppError> {
    if work_order_record.technician_id != Some(requesting_technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let current_timestamp = Utc::now();
    let new_form_id = Uuid::new_v4();

    let part_form_model = new_part_forms::ActiveModel {
        id: Set(new_form_id),
        part_number: Set(add_parts_payload.part_number),
        part_types_id: Set(add_parts_payload.part_types_id),
        model_code: Set(add_parts_payload.model_code),
        serial_number: Set(add_parts_payload.serial_number),
        description: Set(add_parts_payload.description),
        work_order_id: Set(work_order_record.id),
        work_order_number: Set(add_parts_payload.work_order_number),
        status: Set("pending".to_string()),
        created_at: Set(current_timestamp),
        updated_at: Set(current_timestamp),
        deleted_at: Set(None),
    };

    // Create image + link records for each photo object name
    let mut image_models = Vec::new();
    let mut image_link_models = Vec::new();

    for photo_object_name in add_parts_payload.photos {
        let image_id = Uuid::new_v4();
        image_models.push(images::ActiveModel {
            id: Set(image_id),
            object_name: Set(photo_object_name),
            created_at: Set(current_timestamp),
            updated_at: Set(current_timestamp),
            ..Default::default()
        });

        image_link_models.push(new_part_form_image_links::ActiveModel {
            image_id: Set(image_id),
            new_part_form_id: Set(new_form_id),
        });
    }

    Ok(AddPartsEffect {
        part_form_model,
        image_models,
        image_link_models,
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
    fn test_decide_add_parts_success() {
        let technician_id = Uuid::new_v4();
        let work_order_record = dummy_work_order(technician_id);

        let payload = AddPartsRequest {
            part_number: "PN-123".to_string(),
            part_types_id: 1,
            model_code: Some("MC-123".to_string()),
            serial_number: "SN-123".to_string(),
            description: Some("desc".to_string()),
            work_order_number: "WO-123".to_string(),
            photos: vec!["img.png".to_string()],
        };

        let result = decide_add_parts(payload, work_order_record.clone(), technician_id);
        assert!(result.is_ok());
        let add_parts_effect = result.unwrap();

        assert_eq!(add_parts_effect.part_form_model.part_number, Set("PN-123".to_string()));
        assert_eq!(add_parts_effect.part_form_model.work_order_id, Set(work_order_record.id));
        assert_eq!(add_parts_effect.part_form_model.work_order_number, Set("WO-123".to_string()));
        
        assert_eq!(add_parts_effect.image_models.len(), 1);
        assert_eq!(add_parts_effect.image_models[0].object_name, Set("img.png".to_string()));
        assert_eq!(add_parts_effect.image_link_models.len(), 1);
    }

    #[test]
    fn test_decide_add_parts_forbidden() {
        let technician_id = Uuid::new_v4();
        let unauthorized_technician_id = Uuid::new_v4();
        let work_order_record = dummy_work_order(technician_id);

        let payload = AddPartsRequest {
            part_number: "PN-123".to_string(),
            part_types_id: 1,
            model_code: None,
            serial_number: "SN-123".to_string(),
            description: None,
            work_order_number: "WO-123".to_string(),
            photos: vec![],
        };

        let result = decide_add_parts(payload, work_order_record, unauthorized_technician_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(_) => {},
            _ => panic!("Expected Forbidden"),
        }
    }
}


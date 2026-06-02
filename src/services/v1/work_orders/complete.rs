use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::{
        work_orders,
        work_order_closing_forms,
        images,
        work_order_image_links,
        part_changes,
        parts,
        work_order_state_history,
    },
    model::requests::work_orders::complete_request::CompleteWorkOrderRequest,
};

/// Represents the calculated results and side-effects of successfully completing a work order.

#[derive(Debug)]
pub struct CompleteWorkOrderEffect {
    pub closing_form_id: Uuid,
    /// The database model for the closing form containing repair details and signature.
    pub closing_form_model: work_order_closing_forms::ActiveModel,
    /// Database models for any images associated with the completion.
    pub image_models: Vec<images::ActiveModel>,
    pub image_link_models: Vec<work_order_image_links::ActiveModel>,
    /// Database models for auditing changes to individual parts during the service.
    pub part_change_models: Vec<part_changes::ActiveModel>,
    pub part_record_updates: Vec<parts::ActiveModel>,
    /// The database model for the updated work order (transitioned to 'Completed' status).
    pub work_order_model: work_orders::ActiveModel,
    /// The database model for the state history entry recording the completion event.
    pub state_history_model: work_order_state_history::ActiveModel,
    /// Checklist JSON to write to disk (serialized bytes, ready for tokio::fs::write)
    pub checklist_json: Option<Vec<u8>>,
}

/// Determine the outcome of a work order completion request by a technician.
///
/// This function prepares the repair documentation (closing form), records part
/// installations or removals, and transitions the work order to its final status.

pub fn decide_complete_work_order(
    completion_payload: CompleteWorkOrderRequest,
    work_order: work_orders::Model,
    target_completed_status_id: i32,
    technician_id: Uuid,
) -> Result<CompleteWorkOrderEffect, AppError> {
    let current_timestamp = Utc::now();
    let closing_form_id = Uuid::new_v4();

    // 1. Prepare Closing Form
    let closing_form_active_model = work_order_closing_forms::ActiveModel {
        id: Set(closing_form_id),
        product_id: Set(work_order.product_id),
        work_order_id: Set(work_order.id),
        mtm: Set(completion_payload.mtm),
        serial_number: Set(completion_payload.serial_number),
        diagnosis: Set(completion_payload.diagnosis),
        signature_file_name: Set(completion_payload.signature_file_name),
        created_at: Set(current_timestamp),
        updated_at: Set(current_timestamp),
    };

    // 2. Prepare Part Changes and Updates
    let mut part_change_models = Vec::new();
    let mut part_record_updates = Vec::new();
    for pc in completion_payload.part_changes {
        part_change_models.push(part_changes::ActiveModel {
            part_id: Set(pc.part_id),
            work_order_closing_form_id: Set(closing_form_id),
            change_type: Set(pc.change_type.clone()),
            created_at: Set(current_timestamp),
            updated_at: Set(current_timestamp),
            ..Default::default()
        });

        let mut part_update = parts::ActiveModel {
            id: Set(pc.part_id),
            ..Default::default()
        };

        if pc.change_type == "installed" {
            part_update.product_id = Set(Some(work_order.product_id));
            part_update.installation_date = Set(Some(current_timestamp));
        } else if pc.change_type == "uninstalled" {
            part_update.product_id = Set(None);
            part_update.removal_date = Set(Some(current_timestamp));
        }
        part_record_updates.push(part_update);
    }

    // 4. Prepare Checklist as JSON for disk storage
    let checklist_json = completion_payload.checklist.map(|items| {
        serde_json::to_vec_pretty(&items).unwrap_or_else(|_| b"[]".to_vec())
    });

    // 5. Update Work Order
    let mut work_order_active_model: work_orders::ActiveModel = work_order.clone().into();
    work_order_active_model.work_order_status_id = Set(target_completed_status_id);
    work_order_active_model.complete_form_id = Set(Some(closing_form_id));
    work_order_active_model.updated_at = Set(current_timestamp);

    let state_history_active_model = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(target_completed_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(current_timestamp),
    };

    tracing::info!(
        reason = "CompleteWorkOrderSuccess",
        work_order_id = %work_order.id,
        technician_id = %technician_id,
        message = "Successfully decided to complete work order"
    );

    Ok(CompleteWorkOrderEffect {
        closing_form_id,
        closing_form_model: closing_form_active_model,
        image_models: Vec::new(),
        image_link_models: Vec::new(),
        part_change_models,
        part_record_updates,
        work_order_model: work_order_active_model,
        state_history_model: state_history_active_model,
        checklist_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::requests::work_orders::complete_request::{ChecklistResultInput, PartChangeInput};

    fn dummy_work_order() -> work_orders::Model {
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
            technician_id: None,
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
    fn test_decide_complete_work_order_success() {
        let work_order = dummy_work_order();
        let technician_id = Uuid::new_v4();
        let completed_status_id = 4; // Completed

        let request = CompleteWorkOrderRequest {
            mtm: "82K2".to_string(),
            serial_number: "PF3B1234".to_string(),
            part_changes: vec![
                PartChangeInput { part_id: Uuid::new_v4(), change_type: "installed".to_string() }
            ],
            diagnosis: "Repaired screen".to_string(),
            latitude: 10.0,
            longitude: 106.0,
            signature_file_name: "sig.png".to_string(),
            checklist: Some(vec![ChecklistResultInput { id: 1, result: true, notes: None }]),
        };

        let result = decide_complete_work_order(request, work_order, completed_status_id, technician_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order_model.work_order_status_id, Set(completed_status_id));
        assert!(effect.work_order_model.complete_form_id.is_set());
        assert_eq!(effect.closing_form_model.mtm, Set("82K2".to_string()));
        assert_eq!(effect.closing_form_model.serial_number, Set("PF3B1234".to_string()));
        assert_eq!(effect.closing_form_model.diagnosis, Set("Repaired screen".to_string()));
        assert_eq!(effect.closing_form_model.signature_file_name, Set("sig.png".to_string()));

        assert_eq!(effect.part_change_models.len(), 1);
        assert_eq!(effect.part_change_models[0].change_type, Set("installed".to_string()));
        
        assert_eq!(effect.part_record_updates.len(), 1);
        assert!(effect.part_record_updates[0].installation_date.is_set());

        assert!(effect.checklist_json.is_some());
    }
}


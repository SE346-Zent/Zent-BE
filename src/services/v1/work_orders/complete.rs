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

#[derive(Debug)]
pub struct CompleteWorkOrderEffect {
    pub closing_form_id: Uuid,
    pub closing_form: work_order_closing_forms::ActiveModel,
    pub images: Vec<images::ActiveModel>,
    pub image_links: Vec<work_order_image_links::ActiveModel>,
    pub part_changes: Vec<part_changes::ActiveModel>,
    pub part_updates: Vec<parts::ActiveModel>,
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
    /// Checklist JSON to write to disk (serialized bytes, ready for tokio::fs::write)
    pub checklist_json: Option<Vec<u8>>,
}

pub fn decide_complete_work_order(
    req: CompleteWorkOrderRequest,
    work_order: work_orders::Model,
    completed_status_id: i32,
    technician_id: Uuid,
) -> Result<CompleteWorkOrderEffect, AppError> {
    let now = Utc::now();
    let closing_form_id = Uuid::new_v4();

    // 1. Prepare Closing Form
    let closing_form = work_order_closing_forms::ActiveModel {
        id: Set(closing_form_id),
        product_id: Set(work_order.product_id),
        work_order_id: Set(work_order.id),
        mtm: Set(req.mtm),
        serial_number: Set(req.serial_number),
        diagnosis: Set(req.diagnosis),
        signature_file_name: Set(req.signature_file_name),
        created_at: Set(now),
        updated_at: Set(now),
    };

    // 2. Prepare Part Changes and Updates
    let mut part_changes_models = Vec::new();
    let mut part_updates = Vec::new();
    for pc in req.part_changes {
        part_changes_models.push(part_changes::ActiveModel {
            part_id: Set(pc.part_id),
            work_order_closing_form_id: Set(closing_form_id),
            change_type: Set(pc.change_type.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });

        let mut part_update = parts::ActiveModel {
            id: Set(pc.part_id),
            ..Default::default()
        };

        if pc.change_type == "installed" {
            part_update.product_id = Set(Some(work_order.product_id));
            part_update.installation_date = Set(Some(now));
        } else if pc.change_type == "uninstalled" {
            part_update.product_id = Set(None);
            part_update.removal_date = Set(Some(now));
        }
        part_updates.push(part_update);
    }

    // 4. Prepare Checklist as JSON for disk storage
    let checklist_json = req.checklist.map(|items| {
        serde_json::to_vec_pretty(&items).unwrap_or_else(|_| b"[]".to_vec())
    });

    // 5. Update Work Order
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(completed_status_id);
    active_wo.complete_form_id = Set(Some(closing_form_id));
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(completed_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(now),
    };

    Ok(CompleteWorkOrderEffect {
        closing_form_id,
        closing_form,
        images: Vec::new(),
        image_links: Vec::new(),
        part_changes: part_changes_models,
        part_updates,
        work_order: active_wo,
        state_history,
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
        }
    }

    #[test]
    fn test_decide_complete_work_order_success() {
        let wo = dummy_work_order();
        let tech_id = Uuid::new_v4();
        let completed_status_id = 4; // Completed

        let req = CompleteWorkOrderRequest {
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

        let result = decide_complete_work_order(req, wo, completed_status_id, tech_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order.work_order_status_id, Set(completed_status_id));
        assert!(effect.work_order.complete_form_id.is_set());
        assert_eq!(effect.closing_form.mtm, Set("82K2".to_string()));
        assert_eq!(effect.closing_form.serial_number, Set("PF3B1234".to_string()));
        assert_eq!(effect.closing_form.diagnosis, Set("Repaired screen".to_string()));
        assert_eq!(effect.closing_form.signature_file_name, Set("sig.png".to_string()));

        assert_eq!(effect.part_changes.len(), 1);
        assert_eq!(effect.part_changes[0].change_type, Set("installed".to_string()));
        
        assert_eq!(effect.part_updates.len(), 1);
        assert!(effect.part_updates[0].installation_date.is_set());

        assert!(effect.checklist_json.is_some());
    }
}


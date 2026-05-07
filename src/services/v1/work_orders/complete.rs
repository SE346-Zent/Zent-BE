use std::collections::HashMap;
use chrono::{FixedOffset, Utc, Timelike};
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
        overtimes,
        work_order_state_history,
    },
    model::requests::work_orders::complete_request::CompleteWorkOrderRequest,
};

pub struct CompleteWorkOrderEffect {
    pub closing_form: work_order_closing_forms::ActiveModel,
    pub images: Vec<images::ActiveModel>,
    pub image_links: Vec<work_order_image_links::ActiveModel>,
    pub part_changes: Vec<part_changes::ActiveModel>,
    pub part_updates: Vec<parts::ActiveModel>,
    pub overtime: Option<overtimes::ActiveModel>,
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_complete_work_order(
    req: CompleteWorkOrderRequest,
    work_order: work_orders::Model,
    existing_image_links: Vec<work_order_image_links::Model>,
    policies: &HashMap<String, String>,
    completed_status_id: i32,
    technician_id: Uuid,
) -> Result<CompleteWorkOrderEffect, AppError> {
    // 1. Photo Requirements Check
    let phases = ["Pre-check", "Execution", "Completion"];
    for phase in phases {
        let count = existing_image_links.iter().filter(|l| l.phase == phase).count();
        let min_required: usize = policies.get(&format!("min_photos_{}", phase.to_lowercase().replace("-", "_")))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        
        if count < min_required {
            return Err(AppError::BadRequest(format!("Minimum {} photos required for phase {}", min_required, phase)));
        }
    }

    let now = Utc::now();
    let closing_form_id = Uuid::new_v4();

    // 2. Prepare Closing Form
    let closing_form = work_order_closing_forms::ActiveModel {
        id: Set(closing_form_id),
        product_id: Set(work_order.product_id),
        work_order_id: Set(work_order.id),
        mtm: Set(req.mtm),
        serial_number: Set(req.serial_number),
        diagnosis: Set(req.diagnosis),
        signature_url: Set(req.signature_url),
        created_at: Set(now),
        updated_at: Set(now),
    };

    // 3. Prepare Part Changes
    let mut part_changes_models = Vec::new();
    let part_updates = Vec::new();
    for pc in req.part_changes {
        part_changes_models.push(part_changes::ActiveModel {
            part_id: Set(pc.part_id),
            work_order_closing_form_id: Set(closing_form_id),
            change_type: Set(pc.change_type),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });
    }

    // 4. Overtime Logic
    let mut overtime = None;
    let local_now = now.with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap()); // ICT
    if local_now.hour() >= 18 || local_now.hour() < 8 {
        overtime = Some(overtimes::ActiveModel {
            id: Set(Uuid::new_v4()),
            technician_id: Set(technician_id),
            work_order_id: Set(work_order.id),
            overtime_minutes: Set(60), // Placeholder for 1 hour
            created_at: Set(now),
        });
    }

    // 5. Update Work Order
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(completed_status_id);
    active_wo.complete_form_id = Set(Some(closing_form_id));
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        work_order_status_id: Set(completed_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(now),
    };

    Ok(CompleteWorkOrderEffect {
        closing_form,
        images: Vec::new(),
        image_links: Vec::new(),
        part_changes: part_changes_models,
        part_updates,
        overtime,
        work_order: active_wo,
        state_history,
    })
}

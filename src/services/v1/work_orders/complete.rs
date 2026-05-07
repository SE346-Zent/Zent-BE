use std::collections::HashMap;
use chrono::{FixedOffset, Utc, Timelike, Duration};
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
    policies: &HashMap<String, String>,
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

    // 4. Overtime Logic (Calculated from workday_end policy)
    let mut overtime = None;
    let local_now = now.with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap()); // ICT
    
    let workday_start: u32 = policies.get("workday_start")
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let workday_end: u32 = policies.get("workday_end")
        .and_then(|v| v.parse().ok())
        .unwrap_or(18);

    let hour = local_now.hour();
    let mut overtime_minutes = 0;

    if hour >= workday_end {
        // Case A: Finished after workday end on the same day
        let day_end_time = local_now.date_naive()
            .and_hms_opt(workday_end, 0, 0).unwrap()
            .and_local_timezone(FixedOffset::east_opt(7 * 3600).unwrap()).unwrap();
        overtime_minutes = local_now.signed_duration_since(day_end_time).num_minutes();
    } else if hour < workday_start {
        // Case B: Finished after midnight but before next workday start
        let prev_day_end_time = (local_now.date_naive() - Duration::days(1))
            .and_hms_opt(workday_end, 0, 0).unwrap()
            .and_local_timezone(FixedOffset::east_opt(7 * 3600).unwrap()).unwrap();
        overtime_minutes = local_now.signed_duration_since(prev_day_end_time).num_minutes();
    }

    if overtime_minutes > 0 {
        overtime = Some(overtimes::ActiveModel {
            id: Set(Uuid::new_v4()),
            technician_id: Set(technician_id),
            work_order_id: Set(work_order.id),
            overtime_minutes: Set(overtime_minutes as i32),
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

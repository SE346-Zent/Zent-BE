use std::collections::HashMap;
use chrono::{FixedOffset, TimeZone, Utc, Timelike};
use sea_orm::Set;
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::{
        work_orders,
        work_order_closing_forms,
        images,
        work_order_closing_image_links,
        part_changes,
        overtimes,
    },
    model::requests::work_orders::complete_request::CompleteWorkOrderRequest,
};

pub struct CompleteWorkOrderEffect {
    pub closing_form: work_order_closing_forms::ActiveModel,
    pub images: Vec<images::ActiveModel>,
    pub image_links: Vec<work_order_closing_image_links::ActiveModel>,
    pub part_changes: Vec<part_changes::ActiveModel>,
    pub part_updates: Vec<crate::entities::parts::ActiveModel>,
    pub overtime: Option<overtimes::ActiveModel>,
    pub work_order: work_orders::ActiveModel,
}

pub fn decide_complete_work_order(
    req: CompleteWorkOrderRequest,
    work_order: work_orders::Model,
    existing_image_links: Vec<work_order_closing_image_links::Model>,
    policies: &HashMap<String, String>,
    completed_status_id: i32,
) -> Result<CompleteWorkOrderEffect, AppError> {
    // Validate image phases from database records
    let mut phase_counts = HashMap::new();
    phase_counts.insert("pre-disassembly", 0);
    phase_counts.insert("disassembled", 0);
    phase_counts.insert("post-assembly", 0);

    for link in &existing_image_links {
        if let Some(count) = phase_counts.get_mut(link.phase.as_str()) {
            *count += 1;
        }
    }

    for (phase, count) in phase_counts {
        if count < 1 || count > 5 {
            return Err(AppError::BadRequest(format!(
                "Phase '{}' must have between 1 and 5 images (found {} in database). Please upload them before completing.",
                phase, count
            )));
        }
    }

    // Ensure we don't complete an already completed work order
    if work_order.work_order_status_id == completed_status_id {
        return Err(AppError::BadRequest("Work order is already completed".into()));
    }

    let now = Utc::now();
    let closing_form_id = Uuid::new_v4();

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
        ..Default::default()
    };

    let images_models = Vec::new();
    let image_links = Vec::new();

    let mut part_changes_models = Vec::new();
    let mut part_updates = Vec::new();
    for pc in req.part_changes {
        part_changes_models.push(part_changes::ActiveModel {
            part_id: Set(pc.part_id),
            work_order_closing_form_id: Set(closing_form_id),
            change_type: Set(pc.change_type.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        });

        let mut part_update = crate::entities::parts::ActiveModel {
            id: Set(pc.part_id),
            updated_at: Set(now),
            ..Default::default()
        };

        match pc.change_type.as_str() {
            "installed" => {
                part_update.product_id = Set(Some(work_order.product_id));
                part_update.installation_date = Set(Some(now));
            }
            "uninstalled" => {
                part_update.product_id = Set(None);
                part_update.removal_date = Set(Some(now));
            }
            _ => return Err(AppError::BadRequest(format!("Invalid change_type: {}", pc.change_type))),
        }
        part_updates.push(part_update);
    }

    // Overtime calculation
    let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap(); // GMT+7
    let now_local = now.with_timezone(&tz_offset);

    let workday_end: u32 = policies.get("workday_end")
        .and_then(|v| v.parse().ok())
        .unwrap_or(17); // Default to 17 as per policy

    let mut overtime = None;
    let hour = now_local.hour();
    if hour >= workday_end {
        let end_of_workday = now_local.date_naive().and_hms_opt(workday_end, 0, 0).unwrap();
        let end_of_workday_local = tz_offset.from_local_datetime(&end_of_workday).unwrap();
        
        let overtime_minutes = (now_local - end_of_workday_local).num_minutes() as i32;
        if overtime_minutes > 0 {
            overtime = Some(overtimes::ActiveModel {
                id: Set(Uuid::new_v4()),
                technician_id: Set(work_order.technician_id.unwrap_or_default()),
                work_order_id: Set(work_order.id),
                overtime_minutes: Set(overtime_minutes),
                created_at: Set(now),
            });
        }
    }

    let mut active_wo: work_orders::ActiveModel = work_order.into();
    active_wo.work_order_status_id = Set(completed_status_id);
    active_wo.updated_at = Set(now);

    Ok(CompleteWorkOrderEffect {
        closing_form,
        images: images_models,
        image_links,
        part_changes: part_changes_models,
        part_updates,
        overtime,
        work_order: active_wo,
    })
}

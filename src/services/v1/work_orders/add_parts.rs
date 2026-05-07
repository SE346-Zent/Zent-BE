use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{new_part_forms, work_orders},
    model::requests::work_orders::add_parts_request::AddPartsRequest,
};

pub struct AddPartsEffect {
    pub new_part_form: new_part_forms::ActiveModel,
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

    Ok(AddPartsEffect { new_part_form })
}

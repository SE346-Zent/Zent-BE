use std::collections::HashMap;
use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use crate::core::errors::AppError;
use crate::entities::{work_orders, images, work_order_image_links};
use crate::model::requests::media::confirm_upload_request::ConfirmUploadRequest;

pub struct ConfirmUploadEffect {
    pub image: images::ActiveModel,
    pub image_link: work_order_image_links::ActiveModel,
}

pub fn decide_confirm_upload(
    req: ConfirmUploadRequest,
    work_order: &work_orders::Model,
    technician_id: Uuid,
    _target_lat: f64,
    _target_lng: f64,
    object_name: String,
    _policies: &HashMap<String, String>,
) -> Result<ConfirmUploadEffect, AppError> {
    // 1. Security Check
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // 2. Geofencing Check (Optional/Placeholder for now)
    // NOTE: Geofencing is handled in handlers for photos or enforced only at start/complete

    // 3. Prepare Side-Effects
    let now = Utc::now();
    let image_id = Uuid::new_v4();

    let image = images::ActiveModel {
        id: Set(image_id),
        object_name: Set(object_name),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let image_link = work_order_image_links::ActiveModel {
        image_id: Set(image_id),
        work_order_id: Set(work_order.id),
        phase: Set(req.phase),
        latitude: Set(Some(req.latitude)),
        longitude: Set(Some(req.longitude)),
        is_verified: Set(true),
    };

    Ok(ConfirmUploadEffect { image, image_link })
}

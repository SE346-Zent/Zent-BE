use std::collections::HashMap;
use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use crate::core::errors::AppError;
use crate::entities::{work_orders, images, work_order_image_links};
use crate::model::requests::media::confirm_upload_request::ConfirmUploadRequest;
use crate::utils::geo::is_within_geofence;

/// Represents the calculated results and side-effects of a successful media upload confirmation.
pub struct ConfirmUploadEffect {
    /// The database model for the registered image record.
    pub image_model: images::ActiveModel,
    /// The database model linking the image to the specific work order and service phase.
    pub image_link_model: work_order_image_links::ActiveModel,
}

/// Determine the outcome of a media upload confirmation by validating security, time drift, and geofencing.
///
/// This pure function ensures that only the assigned technician can confirm
/// uploads, validates the device's clock drift against server time, and
/// verifies that the upload coordinates are within the site's geofence.
///
/// # Arguments
/// * `confirmation_payload` - The request containing upload metadata (coordinates, phase, device time).
/// * `work_order_record` - The database model representing the associated work order.
/// * `requesting_technician_id` - The unique identifier of the technician performing the upload.
/// * `site_latitude` - The target geofenced latitude of the work site.
/// * `site_longitude` - The target geofenced longitude of the work site.
/// * `uploaded_object_name` - The unique name/path of the object in the storage bucket.
/// * `system_policies` - A map of configuration policies (e.g., geofencing radius, time drift).
///
/// # Returns
/// A result containing the `ConfirmUploadEffect` on success, or an `AppError` for various violations.
pub fn decide_confirm_upload(
    confirmation_payload: ConfirmUploadRequest,
    work_order_record: &work_orders::Model,
    requesting_technician_id: Uuid,
    site_latitude: f64,
    site_longitude: f64,
    uploaded_object_name: String,
    system_policies: &HashMap<String, String>,
) -> Result<ConfirmUploadEffect, AppError> {
    // 1. Security Check
    if work_order_record.technician_id != Some(requesting_technician_id) {
        tracing::warn!(
            error.message = "TechnicianNotAssignedToWorkOrder",
            error.details = "",
            work_order_id = %work_order_record.id,
            assigned_technician_id = ?work_order_record.technician_id,
            requesting_technician_id = %requesting_technician_id,
            message = "Technician is not assigned to this work order"
        );
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // 2. Internet time drift check
    let allowed_drift_minutes: i64 = system_policies
        .get("internet_time_drift_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let current_server_time = Utc::now();
    let drift_seconds = (current_server_time.timestamp() - confirmation_payload.internet_time).abs();
    if drift_seconds > allowed_drift_minutes * 60 {
        tracing::warn!(
            error.message = "DeviceTimeDriftTooLarge",
            error.details = "",
            work_order_id = %work_order_record.id,
            requesting_technician_id = %requesting_technician_id,
            drift_seconds = %drift_seconds,
            allowed_drift_minutes = %allowed_drift_minutes,
            message = "Device time is too far from server time"
        );
        return Err(AppError::BadRequest(format!(
            "Device time is too far from server time ({} seconds drift, max {} minutes allowed). Please sync your device clock and try again.",
            drift_seconds, allowed_drift_minutes
        )));
    }

    // 3. Geofencing Check
    let geofence_radius_meters: f64 = system_policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000.0);

    let is_within_site = is_within_geofence(
        confirmation_payload.latitude,
        confirmation_payload.longitude,
        site_latitude,
        site_longitude,
        geofence_radius_meters,
    );

    if !is_within_site {
        tracing::warn!(
            error.message = "GeofencingViolation",
            error.details = "",
            work_order_id = %work_order_record.id,
            requesting_technician_id = %requesting_technician_id,
            latitude = %confirmation_payload.latitude,
            longitude = %confirmation_payload.longitude,
            site_latitude = %site_latitude,
            site_longitude = %site_longitude,
            geofence_radius_meters = %geofence_radius_meters,
            message = "Geofencing violation: technician is too far from the work site"
        );
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    // 4. Prepare Side-Effects
    let image_id = Uuid::new_v4();

    let image_model = images::ActiveModel {
        id: Set(image_id),
        object_name: Set(uploaded_object_name),
        internet_time: Set(Some(confirmation_payload.internet_time)),
        created_at: Set(current_server_time),
        updated_at: Set(current_server_time),
        ..Default::default()
    };

    let image_link_model = work_order_image_links::ActiveModel {
        image_id: Set(image_id),
        work_order_id: Set(work_order_record.id),
        phase: Set(confirmation_payload.phase),
        latitude: Set(Some(confirmation_payload.latitude)),
        longitude: Set(Some(confirmation_payload.longitude)),
        is_verified: Set(true),
    };

    tracing::info!(
        work_order_id = %work_order_record.id,
        image_id = %image_id,
        requesting_technician_id = %requesting_technician_id,
        message = "Successfully decided to confirm media upload"
    );
    Ok(ConfirmUploadEffect { image_model, image_link_model })
}

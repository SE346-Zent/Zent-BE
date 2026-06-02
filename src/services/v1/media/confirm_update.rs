use std::collections::HashMap;
use sea_orm::Set;
use uuid::Uuid;
use chrono::{Utc, DateTime};
use crate::core::errors::AppError;
use crate::model::requests::media::confirm_update_request::ConfirmUpdateRequest;
use crate::entities::{work_orders, work_order_image_links};
use crate::utils::geo::is_within_geofence;

/// Represents the calculated results and side-effects of a successful media update confirmation.
pub struct ConfirmUpdateEffect {
    /// The unique identifier of the image being updated.
    pub target_image_id: Uuid,
    /// The new unique name/path of the object in OCI storage.
    pub new_object_name: String,
    /// The client-provided Unix timestamp of the update.
    pub device_internet_time: i64,
    /// The server-side timestamp recording when the update was processed.
    pub server_updated_at: DateTime<Utc>,
    /// The database model containing the updated link metadata (location, verification).
    pub image_link_update_model: work_order_image_links::ActiveModel,
}

/// Determine the outcome of a media update request by validating security, time drift, and geofencing.
///
/// This pure function ensures that only the assigned technician can update
/// existing media, validates the device's clock drift against server time,
/// and verifies that the update coordinates are within the site's geofence.
///
/// # Arguments
/// * `update_payload` - The request containing new upload metadata (coordinates, device time).
/// * `work_order_record` - The database model representing the associated work order.
/// * `target_image_id` - The unique identifier of the image to be updated.
/// * `existing_link_record` - The database model of the existing work order image link.
/// * `requesting_technician_id` - The unique identifier of the technician performing the update.
/// * `site_latitude` - The target geofenced latitude of the work site.
/// * `site_longitude` - The target geofenced longitude of the work site.
/// * `new_uploaded_object_name` - The new unique name/path of the object in the storage bucket.
/// * `system_policies` - A map of configuration policies (e.g., geofencing radius, time drift).
///
/// # Returns
/// A result containing the `ConfirmUpdateEffect` on success, or an `AppError` for various violations.
pub fn decide_confirm_update(
    update_payload: ConfirmUpdateRequest,
    work_order_record: &work_orders::Model,
    target_image_id: Uuid,
    existing_link_record: work_order_image_links::Model,
    requesting_technician_id: Uuid,
    site_latitude: f64,
    site_longitude: f64,
    new_uploaded_object_name: String,
    system_policies: &HashMap<String, String>,
) -> Result<ConfirmUpdateEffect, AppError> {
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
    let drift_seconds = (current_server_time.timestamp() - update_payload.internet_time).abs();
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
        update_payload.latitude,
        update_payload.longitude,
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
            latitude = %update_payload.latitude,
            longitude = %update_payload.longitude,
            site_latitude = %site_latitude,
            site_longitude = %site_longitude,
            geofence_radius_meters = %geofence_radius_meters,
            message = "Geofencing violation: technician is too far from the work site"
        );
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    // 4. Prepare Side-Effects
    let mut image_link_update_model: work_order_image_links::ActiveModel = existing_link_record.into();
    image_link_update_model.latitude = Set(Some(update_payload.latitude));
    image_link_update_model.longitude = Set(Some(update_payload.longitude));
    image_link_update_model.is_verified = Set(true);

    tracing::info!(
        work_order_id = %work_order_record.id,
        target_image_id = %target_image_id,
        requesting_technician_id = %requesting_technician_id,
        message = "Successfully decided to confirm media update"
    );
    Ok(ConfirmUpdateEffect {
        target_image_id,
        new_object_name: new_uploaded_object_name,
        device_internet_time: update_payload.internet_time,
        server_updated_at: current_server_time,
        image_link_update_model,
    })
}

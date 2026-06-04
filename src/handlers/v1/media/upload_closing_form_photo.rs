use axum::{extract::{State, Path, Multipart}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, TransactionTrait, ActiveModelTrait};
use uuid::Uuid;
use crate::core::errors::AppError;
use crate::core::lookup_tables::LookupTables;
use crate::extractor::auth_user::AuthUser;
use crate::utils::{oci, geocoding};
use crate::entities::work_orders;
use crate::services::v1::media::confirm_upload;
use crate::model::responses::base::ApiResponse;
use crate::model::requests::media::confirm_upload_request::ConfirmUploadRequest;

#[utoipa::path(
    post,
    path = "/api/v1/media/work_orders/{id}/closing_form/photos",
    request_body(content_type = "multipart/form-data", description = "Image file and metadata (latitude, longitude, phase)"),
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the work order")
    ),
    responses(
        (status = 200, description = "Photo uploaded successfully"),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden/Geofencing violation"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
/// Handle multipart/form-data requests to upload a closing form photo and record its location.
///
/// This handler extracts the image data and metadata (geolocation, phase, device time),
/// geocodes the work site address, uploads the image to OCI Object Storage,
/// validates geofencing and time drift rules, and persists the metadata in MySQL.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user (must be the assigned technician).
/// * `db_connection` - Shared database connection pool.
/// * `lookup_tables` - Shared in-memory reference tables (for policies).
/// * `work_order_id` - The unique ID of the work order from the URL path.
/// * `multipart_payload` - The raw multipart request body.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn upload_closing_form_photo(
    Extension(authenticated_user): Extension<AuthUser>,
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Path(work_order_id): Path<Uuid>,
    mut multipart_payload: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut uploaded_file_data = None;
    let mut file_content_type = String::new();
    let mut original_file_name = String::new();
    let mut device_latitude = None;
    let mut device_longitude = None;
    let mut service_phase = String::new();
    let mut device_internet_time = None;

    while let Some(field) = multipart_payload.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "file" => {
                file_content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                original_file_name = field.file_name().unwrap_or("upload.jpg").to_string();
                uploaded_file_data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "latitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                device_latitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "longitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                device_longitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "phase" => {
                service_phase = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            "internet_time" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                device_internet_time = Some(val.parse::<i64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            _ => {}
        }
    }

    let file_bytes = uploaded_file_data.ok_or_else(|| AppError::BadRequest("Please select a file to upload".to_string()))?;
    let latitude = device_latitude.ok_or_else(|| AppError::BadRequest("Location latitude is required".to_string()))?;
    let longitude = device_longitude.ok_or_else(|| AppError::BadRequest("Location longitude is required".to_string()))?;
    let internet_time = device_internet_time.ok_or_else(|| AppError::BadRequest("Device internet time is required".to_string()))?;

    if service_phase.is_empty() {
        return Err(AppError::BadRequest("Service phase is required".to_string()));
    }

    if internet_time.is_negative() {
        return Err(AppError::BadRequest("Device internet time must be a valid timestamp".to_string()));
    }

    let work_order_record = work_orders::Entity::find_by_id(work_order_id)
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let site_location = geocoding::geocode_address(
        &work_order_record.address, &work_order_record.ward, &work_order_record.province, &work_order_record.country,
    ).await?;

    let file_extension = original_file_name.split('.').next_back().unwrap_or("jpg");
    let generated_unique_name = format!(
        "{}/wo_closing/{}.{}", work_order_id, chrono::Utc::now().timestamp(), file_extension
    );

    let oci_object_name = oci::upload_object(&generated_unique_name, file_bytes.to_vec(), &file_content_type).await?;

    let confirmation_payload = ConfirmUploadRequest { unique_file_name: generated_unique_name, latitude, longitude, phase: service_phase, internet_time };

    let confirmation_effect = confirm_upload::decide_confirm_upload(
        confirmation_payload, &work_order_record, authenticated_user.user.id,
        site_location.lat, site_location.lng, oci_object_name, &lookup_tables.policies,
    )?;

    db_connection.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            confirmation_effect.image_model.insert(txn).await?;
            confirmation_effect.image_link_model.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|err| match err {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Photo uploaded successfully")))
}

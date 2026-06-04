use axum::{extract::{State, Path, Multipart}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, TransactionTrait, ActiveModelTrait, Set};
use uuid::Uuid;
use crate::core::errors::AppError;
use crate::core::lookup_tables::LookupTables;
use crate::extractor::auth_user::AuthUser;
use crate::utils::{oci, geocoding};
use crate::entities::{work_orders, work_order_image_links, images};
use crate::services::v1::media::confirm_update;
use crate::model::responses::base::ApiResponse;
use crate::model::requests::media::confirm_update_request::ConfirmUpdateRequest;

#[utoipa::path(
    patch,
    path = "/api/v1/media/work_orders/{id}/closing_form/photos/{image_id}",
    request_body(content_type = "multipart/form-data", description = "New image file and metadata (latitude, longitude)"),
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the work order"),
        ("image_id" = Uuid, Path, description = "The unique identifier of the image to update")
    ),
    responses(
        (status = 200, description = "Photo updated successfully"),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden/Geofencing violation"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
/// Handle multipart/form-data requests to update an existing closing form photo with a new image and location.
///
/// This handler extracts the new image data and metadata, geocodes the work site,
/// uploads the new image to OCI, validates security and geofencing rules,
/// and performs a multi-table database transaction to update the image and link records.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user (must be the assigned technician).
/// * `db_connection` - Shared database connection pool.
/// * `lookup_tables` - Shared in-memory reference tables (for policies).
/// * `work_order_id_and_image_id` - A tuple containing the work order ID and the image ID from the URL path.
/// * `multipart_payload` - The raw multipart request body.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn update_closing_form_photo(
    Extension(authenticated_user): Extension<AuthUser>,
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Path((work_order_id, image_id)): Path<(Uuid, Uuid)>,
    mut multipart_payload: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut uploaded_file_data = None;
    let mut file_content_type = String::new();
    let mut original_file_name = String::new();
    let mut device_latitude = None;
    let mut device_longitude = None;
    let mut device_internet_time = None;

    while let Some(field) = multipart_payload.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "file" => {
                file_content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                original_file_name = field.file_name().unwrap_or("update.jpg").to_string();
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

    let work_order_record = work_orders::Entity::find_by_id(work_order_id)
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let existing_link_record = work_order_image_links::Entity::find_by_id((image_id, work_order_id))
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Image link not found".to_string()))?;

    let site_location = geocoding::geocode_address(
        &work_order_record.address, &work_order_record.ward, &work_order_record.province, &work_order_record.country,
    ).await?;

    let file_extension = original_file_name.split('.').next_back().unwrap_or("jpg");
    let generated_unique_name = format!(
        "{}/wo_closing_update/{}.{}", work_order_id, chrono::Utc::now().timestamp(), file_extension
    );

    let oci_object_name = oci::upload_object(&generated_unique_name, file_bytes.to_vec(), &file_content_type).await?;

    let update_payload = ConfirmUpdateRequest { unique_file_name: generated_unique_name, latitude, longitude, internet_time };

    let confirmation_effect = confirm_update::decide_confirm_update(
        update_payload, &work_order_record, image_id, existing_link_record, authenticated_user.user.id,
        site_location.lat, site_location.lng, oci_object_name, &lookup_tables.policies,
    )?;

    db_connection.transaction::<_, (), AppError>(|txn| {
        let image_id = confirmation_effect.target_image_id;
        let object_name = confirmation_effect.new_object_name;
        let updated_at = confirmation_effect.server_updated_at;
        let internet_time = confirmation_effect.device_internet_time;
        let link_update = confirmation_effect.image_link_update_model;
        Box::pin(async move {
            let mut img_active = images::ActiveModel { id: Set(image_id), ..Default::default() };
            img_active.object_name = Set(object_name);
            img_active.internet_time = Set(Some(internet_time));
            img_active.updated_at = Set(updated_at);
            img_active.update(txn).await?;
            link_update.update(txn).await?;
            Ok(())
        })
    }).await.map_err(|err| match err {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Photo updated successfully")))
}

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
    responses(
        (status = 200, description = "Photo uploaded successfully"),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden/Geofencing violation"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_closing_form_photo(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::new();
    let mut file_name = String::new();
    let mut latitude = None;
    let mut longitude = None;
    let mut phase = String::new();
    let mut internet_time = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                file_name = field.file_name().unwrap_or("upload.jpg").to_string();
                file_data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "latitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                latitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "longitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                longitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "phase" => {
                phase = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            "internet_time" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                internet_time = Some(val.parse::<i64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::BadRequest("File is missing".to_string()))?;
    let latitude = latitude.ok_or_else(|| AppError::BadRequest("Latitude is missing".to_string()))?;
    let longitude = longitude.ok_or_else(|| AppError::BadRequest("Longitude is missing".to_string()))?;
    let internet_time = internet_time.ok_or_else(|| AppError::BadRequest("Internet time is missing".to_string()))?;

    if phase.is_empty() {
        return Err(AppError::BadRequest("Phase is missing".to_string()));
    }

    if internet_time.is_negative() {
        return Err(AppError::BadRequest("Internet time must be a positive integer".to_string()));
    }

    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let target_location = geocoding::geocode_address(
        &work_order.address, &work_order.city, &work_order.province, &work_order.country,
    ).await?;

    let extension = file_name.split('.').last().unwrap_or("jpg");
    let unique_file_name = format!(
        "{}/wo_closing/{}.{}", id, chrono::Utc::now().timestamp(), extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    let payload = ConfirmUploadRequest { unique_file_name, latitude, longitude, phase, internet_time };

    let effect = confirm_upload::decide_confirm_upload(
        payload, &work_order, auth.user.id,
        target_location.lat, target_location.lng, object_name, &luts.policies,
    )?;

    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.image.insert(txn).await?;
            effect.image_link.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Photo uploaded successfully")))
}

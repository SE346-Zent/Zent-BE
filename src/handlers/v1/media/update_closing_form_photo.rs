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
    responses(
        (status = 200, description = "Photo updated successfully"),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden/Geofencing violation"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_closing_form_photo(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path((id, image_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::new();
    let mut file_name = String::new();
    let mut latitude = None;
    let mut longitude = None;
    let mut internet_time = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                file_name = field.file_name().unwrap_or("update.jpg").to_string();
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
    let internet_time = internet_time.ok_or_else(|| AppError::BadRequest("internet_time is missing".to_string()))?;

    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let link = work_order_image_links::Entity::find_by_id((image_id, id))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Image link not found".to_string()))?;

    let target_location = geocoding::geocode_address(
        &work_order.address, &work_order.city, &work_order.province, &work_order.country,
    ).await?;

    let extension = file_name.split('.').next_back().unwrap_or("jpg");
    let unique_file_name = format!(
        "{}/wo_closing_update/{}.{}", id, chrono::Utc::now().timestamp(), extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    let payload = ConfirmUpdateRequest { unique_file_name, latitude, longitude, internet_time };

    let effect = confirm_update::decide_confirm_update(
        payload, &work_order, image_id, link, auth.user.id,
        target_location.lat, target_location.lng, object_name, &luts.policies,
    )?;

    db.transaction::<_, (), AppError>(|txn| {
        let image_id = effect.image_id;
        let object_name = effect.object_name;
        let updated_at = effect.updated_at;
        let link_update = effect.link_update;
        Box::pin(async move {
            let mut img_active = images::ActiveModel { id: Set(image_id), ..Default::default() };
            img_active.object_name = Set(object_name);
            img_active.internet_time = Set(Some(effect.internet_time));
            img_active.updated_at = Set(updated_at);
            img_active.update(txn).await?;
            link_update.update(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Photo updated successfully")))
}

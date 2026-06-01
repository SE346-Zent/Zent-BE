use axum::{extract::{State, Multipart}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, Set};
use uuid::Uuid;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::utils::oci;
use crate::model::responses::base::ApiResponse;
use crate::entities::{images, chat_room_image_links};
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadResponse {
    /// Unique object name / file name the frontend uses with PAR_READ to fetch from OCI.
    pub object_name: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/chat/attachments",
    request_body(content_type = "multipart/form-data", description = "File and room_id to attach to"),
    responses(
        (status = 200, description = "File uploaded successfully", body = ApiResponse<AttachmentUploadResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    tag = "chat",
    security(("bearer_auth" = []))
)]
pub async fn upload_attachment(
    Extension(_auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<AttachmentUploadResponse>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::from("image/jpeg");
    let mut file_name = String::from("upload.jpg");
    let mut room_id: Option<Uuid> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                file_name = field.file_name().unwrap_or("upload.jpg").to_string();
                file_data = Some(field.bytes().await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "room_id" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                room_id = Some(Uuid::parse_str(&val).map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?);
            }
            _ => {}
        }
    }

    let file_data = file_data
        .ok_or_else(|| AppError::BadRequest("Please select a file to upload".to_string()))?;

    let room_id = room_id
        .ok_or_else(|| AppError::BadRequest("Room ID is required".to_string()))?;

    let extension = file_name.split('.').last().unwrap_or("jpg");
    let object_name = format!(
        "chat_attachments/{}/{}.{}",
        room_id,
        chrono::Utc::now().timestamp_millis(),
        extension
    );

    // Upload to OCI
    oci::upload_object(&object_name, file_data.to_vec(), &content_type).await?;

    let now = chrono::Utc::now();
    let image_id = Uuid::new_v4();

    // Insert into images table
    let image = images::ActiveModel {
        id: Set(image_id),
        object_name: Set(object_name.clone()),
        internet_time: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };
    image.insert(db.as_ref()).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save image record: {}", e)))?;

    // Insert link between chat room and image
    let link = chat_room_image_links::ActiveModel {
        image_id: Set(image_id),
        room_id: Set(room_id),
        created_at: Set(now),
    };
    link.insert(db.as_ref()).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save image link: {}", e)))?;

    let response = AttachmentUploadResponse { object_name };

    Ok(Json(ApiResponse::success(200, "File uploaded successfully", response)))
}

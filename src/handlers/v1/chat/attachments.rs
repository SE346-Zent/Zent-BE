use axum::{extract::{State, Multipart}, Json, Extension};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::utils::oci;
use crate::model::responses::base::ApiResponse;
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadResponse {
    pub url: String,
    pub object_name: String,
}

#[utoipa::path(
    post,
    path = "/api/chat/attachments",
    request_body(content_type = "multipart/form-data", description = "File to upload"),
    responses(
        (status = 200, description = "File uploaded successfully", body = ApiResponse<AttachmentUploadResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_attachment(
    Extension(_auth): Extension<AuthUser>,
    State(_db): State<Arc<DatabaseConnection>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<AttachmentUploadResponse>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::from("image/jpeg");
    let mut file_name = String::from("upload.jpg");

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
            _ => {}
        }
    }

    let file_data = file_data
        .ok_or_else(|| AppError::BadRequest("File is missing".to_string()))?;

    let extension = file_name.split('.').last().unwrap_or("jpg");
    let unique_file_name = format!(
        "chat_attachments/{}_{}.{}",
        chrono::Utc::now().timestamp_millis(),
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("0000"),
        extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    let config = crate::core::config::AppConfig::get();
    let url = format!("{}{}", config.par_read_work_orders, object_name);

    let response = AttachmentUploadResponse {
        url,
        object_name,
    };

    Ok(Json(ApiResponse::success(200, "File uploaded successfully", response)))
}

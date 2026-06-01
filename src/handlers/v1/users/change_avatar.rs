use axum::{extract::{State, Multipart}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, Set};
use chrono::Utc;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::utils::oci;
use serde::Serialize;

const MAX_AVATAR_SIZE: usize = 5 * 1024 * 1024; // 5MB
const ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAvatarResponse {
    pub avatar_name: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/users/me/avatar",
    request_body(content_type = "multipart/form-data", description = "Avatar image file"),
    responses(
        (status = 200, description = "Avatar updated successfully", body = ApiResponse<ChangeAvatarResponse>),
        (status = 400, description = "Bad Request - Invalid file or too large", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    tag = "users",
    security(("bearer_auth" = []))
)]
pub async fn change_avatar(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<ChangeAvatarResponse>>, AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type = String::from("image/jpeg");
    let mut file_name = String::from("avatar.jpg");

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            file_name = field.file_name().unwrap_or("avatar.jpg").to_string();
            let bytes = field.bytes().await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            file_data = Some(bytes.to_vec());
        }
    }

    let file_data = file_data
        .ok_or_else(|| AppError::BadRequest("Please select a file to upload".to_string()))?;

    // Validate content type
    if !ALLOWED_TYPES.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest("Invalid file type. Allowed formats: JPEG, PNG, WebP".to_string()));
    }

    // Validate file size
    if file_data.len() > MAX_AVATAR_SIZE {
        return Err(AppError::BadRequest("File size exceeds the 5MB limit".to_string()));
    }

    // Generate object name: avatars/{user_id}/{timestamp}.{ext}
    let extension = file_name.split('.').last().unwrap_or("jpg");
    let safe_ext = match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        _ => "jpg",
    };
    let object_name = format!(
        "avatars/{}/{}.{}",
        user.id,
        Utc::now().timestamp_millis(),
        safe_ext
    );

    // Upload to OCI
    oci::upload_object(&object_name, file_data.to_vec(), &content_type).await?;

    // Update user's avatar_url in DB
    let mut active_model: crate::entities::users::ActiveModel = user.into();
    active_model.avatar_url = Set(Some(object_name.clone()));
    active_model.updated_at = Set(Utc::now());
    active_model.update(db.as_ref()).await?;

    Ok(Json(ApiResponse::success(200, "Avatar updated successfully", ChangeAvatarResponse {
        avatar_name: object_name,
    })))
}

use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use mongodb::{bson::doc, Database as MongoDatabase};
use validator::Validate;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::chat::send_message_request::SendMessageRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::chat::message_response::MessageResponse;

#[utoipa::path(
    post, path = "/api/chat/rooms/{id}/messages",
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Message sent", body = ApiResponse<MessageResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn send_message(
    Extension(auth): Extension<AuthUser>,
    State(mongo): State<Arc<MongoDatabase>>,
    Path(room_id): Path<String>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let now = mongodb::bson::DateTime::now();
    let doc = doc! {
        "room_id": &room_id,
        "sender_id": auth.user.id.to_string(),
        "content": payload.content.as_deref().unwrap_or(""),
        "image_url": payload.image_url.as_deref(),
        "reply_to": payload.reply_to.as_deref(),
        "created_at": now,
        "edited_at": mongodb::bson::Bson::Null,
    };

    let col = mongo.collection::<mongodb::bson::Document>("messages");
    let result = col.insert_one(doc).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("MongoDB insert error: {}", e)))?;

    let message_id = result.inserted_id.as_object_id()
        .map(|o| o.to_hex())
        .unwrap_or_default();

    let response = crate::services::v1::chat::map_message_response::map_to_message_response(
        message_id,
        room_id,
        auth.user.id.to_string(),
        auth.user.full_name.clone(),
        payload.content,
        payload.image_url,
        payload.reply_to,
        now.to_string(),
        None,
        vec![],
    );

    Ok(Json(ApiResponse::success(201, "Message sent successfully", response)))
}

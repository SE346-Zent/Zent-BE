use axum::{extract::{Path, State}, Json};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;

use crate::core::errors::{AppError, ErrorResponse};
use crate::entities::{images, new_part_audit_log, new_part_form_image_links, new_part_forms, part_types};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::inventory::new_part_form_detail_response::NewPartFormDetailResponse;
use crate::services::v1::inventory::new_part_forms as new_part_forms_service;

#[utoipa::path(
    get,
    path = "/api/v1/inventory/part-requests/{id}",
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the new part form")
    ),
    responses(
        (status = 200, description = "New part form detail retrieved successfully", body = ApiResponse<NewPartFormDetailResponse>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "New part form not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn new_part_form_detail(
    _auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<NewPartFormDetailResponse>>, AppError> {
    let form = new_part_forms::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("New part form not found".to_string()))?;

    let part_type_name = part_types::Entity::find_by_id(form.part_types_id)
        .one(db.as_ref())
        .await?
        .map(|item| item.part_type_name)
        .unwrap_or_else(|| "Unknown".to_string());

    let photo_urls = new_part_form_image_links::Entity::find()
        .filter(new_part_form_image_links::Column::NewPartFormId.eq(form.id))
        .find_also_related(images::Entity)
        .all(db.as_ref())
        .await?
        .into_iter()
        .filter_map(|(_, image)| image)
        .map(|image| image.object_name)
        .collect::<Vec<_>>();

    let rejection_reason = if form.status.eq_ignore_ascii_case("rejected") {
        new_part_audit_log::Entity::find()
            .filter(new_part_audit_log::Column::NewPartFormId.eq(form.id))
            .filter(new_part_audit_log::Column::Action.eq("denied"))
            .one(db.as_ref())
            .await?
            .and_then(|audit| audit.reason)
    } else {
        None
    };

    let detail = new_part_forms_service::map_detail_response(form, part_type_name, photo_urls, rejection_reason);

    Ok(Json(ApiResponse::success(
        200,
        "New part form retrieved successfully",
        detail,
    )))
}
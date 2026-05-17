use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use uuid::Uuid;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::reject_form_detail_response::RejectFormDetail;
use crate::entities::{work_order_reject_forms, work_order_reject_form_image_links, images};

#[utoipa::path(
    get, path = "/api/v1/work_orders/reject_forms/{id}",
    responses(
        (status = 200, description = "Rejection form detail with photos", body = ApiResponse<RejectFormDetail>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Rejection form not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reject_form_detail(
    _auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<RejectFormDetail>>, AppError> {
    let rf = work_order_reject_forms::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Rejection form not found".to_string()))?;

    // Fetch photos via the link table -> images
    let photo_urls: Vec<String> = work_order_reject_form_image_links::Entity::find()
        .filter(work_order_reject_form_image_links::Column::WorkOrderRejectFormId.eq(id))
        .find_also_related(images::Entity)
        .all(db.as_ref())
        .await?
        .into_iter()
        .filter_map(|(_, img)| img)
        .map(|img| img.object_name)
        .collect();

    let detail = RejectFormDetail {
        id: rf.id,
        approver_id: rf.approver_id,
        approved: rf.approved,
        reason: rf.reason,
        explanation: rf.explanation,
        photo_urls,
        created_at: rf.created_at,
        updated_at: rf.updated_at,
    };

    Ok(Json(ApiResponse::success(200, "Rejection form detail retrieved successfully", detail)))
}

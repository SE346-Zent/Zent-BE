use axum::{extract::{Query, State}, Json};
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;

use crate::core::errors::{AppError, ErrorResponse};
use crate::entities::{new_part_forms, part_types};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::new_part_form_query::NewPartFormQuery;
use crate::model::requests::pagination::PaginationRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::inventory::new_part_form_list_response::{NewPartFormListResponse};
use crate::model::responses::pagination::PaginationResponse;
use crate::services::v1::inventory::new_part_forms as new_part_forms_service;

fn normalize_status_filter(status: &str) -> Option<&'static str> {
    match status.trim().to_lowercase().as_str() {
        "pending" => Some("pending"),
        "approved" => Some("approved"),
        "rejected" | "denied" => Some("rejected"),
        _ => None,
    }
}

fn status_condition(status: &str) -> Condition {
    match status {
        "pending" => Condition::all().add(new_part_forms::Column::Status.eq("pending")),
        "approved" => Condition::all().add(new_part_forms::Column::Status.eq("approved")),
        "rejected" => Condition::any()
            .add(new_part_forms::Column::Status.eq("rejected"))
            .add(new_part_forms::Column::Status.eq("denied")),
        _ => Condition::all(),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/inventory/part-requests",
    params(NewPartFormQuery),
    responses(
        (status = 200, description = "New part forms retrieved successfully", body = ApiResponse<NewPartFormListResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn new_part_form_list(
    _auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<NewPartFormQuery>,
) -> Result<Json<ApiResponse<NewPartFormListResponse>>, AppError> {
    let PaginationRequest { page, limit } = query.pagination;
    let mut base_query = new_part_forms::Entity::find()
        .filter(new_part_forms::Column::DeletedAt.is_null());

    if let Some(status) = query.status.as_deref() {
        let normalized = normalize_status_filter(status)
            .ok_or_else(|| AppError::BadRequest(format!("Invalid status '{}'; expected pending, approved, or rejected", status)))?;
        base_query = base_query.filter(status_condition(normalized));
    }

    if let Some(search) = query.q.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        let like = format!("%{}%", search);
        base_query = base_query.filter(
            Condition::any()
                .add(new_part_forms::Column::PartNumber.like(&like))
                .add(new_part_forms::Column::WorkOrderNumber.like(&like)),
        );
    }

    let rows: Vec<(new_part_forms::Model, Option<part_types::Model>)> = base_query
        .find_also_related(part_types::Entity)
        .order_by_desc(new_part_forms::Column::CreatedAt)
        .all(db.as_ref())
        .await?;
    let (response, total_records) = new_part_forms_service::map_list_response(rows, page, limit);
    let pagination = PaginationResponse::new(limit, page, total_records);

    Ok(Json(ApiResponse::success_with_meta(
        200,
        "New part forms retrieved successfully",
        response,
        pagination,
    )))
}
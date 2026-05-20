use axum::{extract::{State, Query}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait, QuerySelect};
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::work_orders::reject_form_query::RejectFormQuery;
use crate::model::requests::pagination::PaginationRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::responses::work_orders::reject_form_list_response::RejectFormListItem;
use crate::entities::{work_orders as work_orders_ent, work_order_reject_forms, users};

#[utoipa::path(
    get, path = "/api/v1/work_orders/reject_forms", params(RejectFormQuery),
    responses(
        (status = 200, description = "List of rejection forms", body = ApiResponse<Vec<RejectFormListItem>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reject_form_list(
    _auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<RejectFormQuery>,
) -> Result<Json<ApiResponse<Vec<RejectFormListItem>>>, AppError> {
    let PaginationRequest { page, limit } = query.pagination;

    // Base query: work orders that have a reject form
    let base_query = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::RejectFormId.is_not_null())
        .order_by_desc(work_orders_ent::Column::CreatedAt);

    let total_records = base_query.clone().count(db.as_ref()).await?;

    let rows: Vec<(work_orders_ent::Model, Option<work_order_reject_forms::Model>)> = base_query
        .find_also_related(work_order_reject_forms::Entity)
        .offset((page - 1) * limit)
        .limit(limit)
        .all(db.as_ref())
        .await?;

    // Collect technician IDs and batch-fetch users
    let tech_ids: Vec<uuid::Uuid> = rows.iter()
        .filter_map(|(wo, _)| wo.technician_id)
        .collect();

    let tech_users = if tech_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        users::Entity::find()
            .filter(users::Column::Id.is_in(tech_ids))
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(|u| (u.id, u))
            .collect::<std::collections::HashMap<_, _>>()
    };

    let data: Vec<RejectFormListItem> = rows
        .into_iter()
        .filter_map(|(wo, rf)| {
            let rf = rf?;
            let tech = wo.technician_id.and_then(|tid| tech_users.get(&tid).cloned());
            Some(crate::services::v1::work_orders::reject_forms::map_to_reject_form_list_item(wo, rf, tech))
        })
        .collect();

    let pagination = PaginationResponse::new(limit, page, total_records);

    Ok(Json(ApiResponse::success_with_meta(200, "Rejection forms retrieved successfully", data, pagination)))
}

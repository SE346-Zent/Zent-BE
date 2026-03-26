use axum::{
    extract::{Query, State},
    Json, Router, routing::get,
};
use sea_orm::DatabaseConnection;

use crate::{
    extractor::auth_user::AuthUser,
    model::{
        requests::work_order::{my_work_order_query::WorkOrderQuery, work_order_list_query::WorkOrderListQuery},
        responses::{
            error::{AppError, ErrorResponse},
            work_order::work_order_response::{WorkOrderListResponse, WorkOrderResponse},
        },
    },
    services::v1::work_order::my_work_order_service::{get_my_work_order_service, get_my_work_orders_service},
};

#[utoipa::path(
    get,
    path = "/api/v1/work_order/my_work_order",
    params(WorkOrderQuery),
    responses(
        (status = 200, description = "Retrieve My Work Order", body = WorkOrderResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized JWT Validation", body = ErrorResponse),
        (status = 404, description = "Work Order Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn get_my_work_order(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderQuery>,
    _auth: AuthUser,
) -> Result<Json<WorkOrderResponse>, AppError> {
    let result = get_my_work_order_service(db, query.id).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/work_order/my_work_orders",
    params(WorkOrderListQuery),
    responses(
        (status = 200, description = "Retrieve My Work Orders", body = WorkOrderListResponse),
        (status = 400, description = "Bad Request Query Parameters", body = ErrorResponse),
        (status = 401, description = "Unauthorized Token", body = ErrorResponse),
        (status = 500, description = "Internal Platform Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn get_my_work_orders(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderListQuery>,
    _auth: AuthUser,
) -> Result<Json<WorkOrderListResponse>, AppError> {
    let result = get_my_work_orders_service(db, query).await?;
    Ok(Json(result))
}

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/my_work_order", get(get_my_work_order))
        .route("/my_work_orders", get(get_my_work_orders))
}

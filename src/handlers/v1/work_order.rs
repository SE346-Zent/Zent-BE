use axum::{
    extract::{Query, State},
    Json, Router, routing::{get, post},
};
use sea_orm::DatabaseConnection;

use crate::{
    extractor::auth_user::AuthUser,
    model::{
        requests::work_order::{
            my_work_order_query::WorkOrderQuery,
            work_order_list_query::WorkOrderListQuery,
            create_work_order_request::CreateWorkOrderRequest,
            create_closing_form_request::CreateClosingFormRequest,
        },
        responses::{
            error::{AppError, ErrorResponse},
            work_order::work_order_detail_response::WorkOrderDetailResponse,
            work_order::work_order_list_item_response::WorkOrderListResponse,
            work_order::closing_form_response::ClosingFormResponse,
            warranty::warranty_detail_response::WarrantyDetailResponse,
        },
    },
    services::v1::work_order::{
        my_work_order_service::{get_my_work_order_service, get_my_work_orders_service},
        create_work_order_service::{create_work_order_service, create_closing_form_service},
        warranty_service::get_warranty_service,
    },
};

use crate::model::requests::warranty::warranty_detail_query::WarrantyDetailQuery;

#[utoipa::path(
    get,
    path = "/api/v1/work_order/my_work_order",
    params(WorkOrderQuery),
    responses(
        (status = 200, description = "Retrieve My Work Order", body = WorkOrderDetailResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized JWT Validation", body = ErrorResponse),
        (status = 404, description = "Work Order Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_my_work_order(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderQuery>,
    _auth: AuthUser,
) -> Result<Json<WorkOrderDetailResponse>, AppError> {
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
pub async fn get_my_work_orders(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderListQuery>,
    _auth: AuthUser,
) -> Result<Json<WorkOrderListResponse>, AppError> {
    let result = get_my_work_orders_service(db, query).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/work_order/create",
    request_body = CreateWorkOrderRequest,
    responses(
        (status = 201, description = "Work Order Created", body = WorkOrderDetailResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_work_order(
    State(db): State<DatabaseConnection>,
    _auth: AuthUser,
    Json(body): Json<CreateWorkOrderRequest>,
) -> Result<Json<WorkOrderDetailResponse>, AppError> {
    let result = create_work_order_service(db, body).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/work_order/closing_form",
    request_body = CreateClosingFormRequest,
    responses(
        (status = 201, description = "Closing Form Created", body = ClosingFormResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_closing_form(
    State(db): State<DatabaseConnection>,
    _auth: AuthUser,
    Json(body): Json<CreateClosingFormRequest>,
) -> Result<Json<ClosingFormResponse>, AppError> {
    let result = create_closing_form_service(db, body).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/work_order/warranty",
    params(WarrantyDetailQuery),
    responses(
        (status = 200, description = "Retrieve Warranty", body = WarrantyDetailResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Warranty Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_warranty(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WarrantyDetailQuery>,
    _auth: AuthUser,
) -> Result<Json<WarrantyDetailResponse>, AppError> {
    let result = get_warranty_service(db, query.warranty_id).await?;
    Ok(Json(result))
}

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/my_work_order", get(get_my_work_order))
        .route("/my_work_orders", get(get_my_work_orders))
        .route("/create", post(create_work_order))
        .route("/closing_form", post(create_closing_form))
        .route("/warranty", get(get_warranty))
}

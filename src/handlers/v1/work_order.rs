use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;

use crate::{
    extractor::auth_user::AuthUser,
    model::{
        requests::work_order::{my_work_order_query::WorkOrderQuery, work_order_list_query::WorkOrderListQuery},
        responses::error::AppError,
        responses::work_order::work_order_response::{WorkOrderListResponse, WorkOrderResponse},
    },
    services::v1::work_order::my_work_order_service::{get_my_work_order_service, get_my_work_orders_service},
};

pub async fn get_my_work_order(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderQuery>,
    _auth: AuthUser,
) -> Result<Json<WorkOrderResponse>, AppError> {
    let result = get_my_work_order_service(db, query.id).await?;
    Ok(Json(result))
}

pub async fn get_my_work_orders(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderListQuery>,
    _auth: AuthUser,
) -> Result<Json<WorkOrderListResponse>, AppError> {
    let result = get_my_work_orders_service(db, query).await?;
    Ok(Json(result))
}

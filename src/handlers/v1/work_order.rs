use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;

use crate::{
    model::auth::jwt_claims::Claims,
    model::{
        requests::work_order::my_work_order_query::WorkOrderQuery,
        responses::error::AppError,
        responses::work_order::work_order_response::{WorkOrderListResponse, WorkOrderResponse},
    },
    services::v1::work_order::my_work_order_service::{get_my_work_order_service, get_my_work_orders_service},
};

pub async fn get_my_work_order(
    State(db): State<DatabaseConnection>,
    Query(query): Query<WorkOrderQuery>,
    _claims: Claims,
) -> Result<Json<WorkOrderResponse>, AppError> {
    let result = get_my_work_order_service(db, query.id).await?;
    Ok(Json(result))
}

pub async fn get_my_work_orders(
    State(db): State<DatabaseConnection>,
    _claims: Claims,
) -> Result<Json<WorkOrderListResponse>, AppError> {
    let result = get_my_work_orders_service(db).await?;
    Ok(Json(result))
}

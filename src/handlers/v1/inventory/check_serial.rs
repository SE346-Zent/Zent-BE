use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::model::requests::inventory::check_serial_request::CheckSerialRequest;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::check_serial::check_serial_exists;

/// Validate if a product serial number exists in the catalog.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/products/check-serial",
    request_body = CheckSerialRequest,
    responses(
        (status = 200, description = "Serial validation status returned successfully", body = ApiResponse<bool>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn check_serial(
    State(state): State<AppState>,
    Json(payload): Json<CheckSerialRequest>,
) -> Result<Json<ApiResponse<bool>>, AppError> {
    let zeus_prod = state.zeus_client.find_product_by_serial(&payload.serial_number).await?;
    let exists = check_serial_exists(&payload.serial_number, &zeus_prod);

    Ok(Json(ApiResponse::success(
        200,
        "Serial checked successfully",
        exists,
    )))
}

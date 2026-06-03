use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::lookup_tables::LookupTables;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::technician_metrics_response::TechnicianMetricsResponse;

#[utoipa::path(
    get, path = "/api/v1/work_orders/technician/metrics",
    tag = "work_orders",
    responses(
        (status = 200, description = "Technician metrics retrieved successfully", body = ApiResponse<TechnicianMetricsResponse>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_technician_metrics(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
) -> Result<Json<ApiResponse<TechnicianMetricsResponse>>, AppError> {
    if auth.role.name != "Technician" {
        return Err(AppError::Forbidden("Only technicians can view their metrics".to_string()));
    }

    let closed_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    let snapshot = super::get_cached_technician_stats(
        db.as_ref(),
        &valkey_client,
        auth.user.id,
        &[closed_status_id],
    ).await?;

    Ok(Json(ApiResponse::success(
        200,
        "Technician metrics retrieved successfully",
        TechnicianMetricsResponse {
            active_jobs: snapshot.active_jobs,
            overall_rating: snapshot.average_rating(),
        },
    )))
}

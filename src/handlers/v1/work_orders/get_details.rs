use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::details_response::WorkOrderDetails;
use crate::services::v1::work_orders::get_details as get_svc;
use crate::entities::{products, work_orders as work_orders_ent, work_order_symptoms};

#[utoipa::path(
    get, path = "/api/v1/work_orders/{id}",
    responses(
        (status = 200, description = "Work order details", body = ApiResponse<WorkOrderDetails>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_details(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<WorkOrderDetails>>, AppError> {
    let mut conn_opt = None;
    let cache_key = format!("cache:work_order:{}", id);
    if let Some(client) = valkey_client.as_ref() {
        use redis::AsyncCommands;
        let mut conn = client.get_connection();
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(details) = serde_json::from_str::<WorkOrderDetails>(&cached_json) {
                // Check permissions even if cached
                check_wo_permissions(&auth, &details.work_order_number, details.technician_id, details.customer_id, &details.province)?;
                return Ok(Json(ApiResponse::success(200, "Work order details retrieved successfully", details)));
            }
        }
        conn_opt = Some(conn);
    }
    let result = work_orders_ent::Entity::find_by_id(id)
        .find_also_related(products::Entity).find_also_related(work_order_symptoms::Entity)
        .one(db.as_ref()).await?;
    let (wo, product, symptom) = result.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;
    
    // Check permissions
    check_wo_permissions(&auth, &wo.work_order_number, wo.technician_id, wo.customer_id, &wo.province)?;

    let details = get_svc::decide_get_details(wo, product, symptom, &lookup_tables);
    if let Some(mut conn) = conn_opt {
        use redis::AsyncCommands;
        if let Ok(cached_val) = serde_json::to_string(&details) { let _: () = conn.set_ex(&cache_key, cached_val, 600).await.unwrap_or_default(); }
    }
    Ok(Json(ApiResponse::success(200, "Work order details retrieved successfully", details)))
}

fn check_wo_permissions(
    auth: &AuthUser,
    wo_number: &str,
    tech_id: Option<Uuid>,
    customer_id: Uuid,
    province: &str,
) -> Result<(), AppError> {
    match auth.role.name.as_str() {
        "SuperAdmin" => Ok(()),
        "Admin" => {
            let admin_province = auth.user.province.as_deref().unwrap_or("");
            if admin_province != province {
                return Err(AppError::Forbidden(format!("You do not have permission to view work orders in province '{}'", province)));
            }
            Ok(())
        }
        "Technician" => {
            if tech_id != Some(auth.user.id) {
                return Err(AppError::Forbidden(format!("Work order {} is not assigned to you", wo_number)));
            }
            Ok(())
        }
        "Customer" => {
            if customer_id != auth.user.id {
                return Err(AppError::Forbidden(format!("Work order {} does not belong to you", wo_number)));
            }
            Ok(())
        }
        _ => Err(AppError::Forbidden("Role not recognized".to_string())),
    }
}

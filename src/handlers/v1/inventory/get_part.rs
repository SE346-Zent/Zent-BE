use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::inventory::part_detail_response::PartDetailResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::get_part::{self, PartWithRelations};
use crate::services::v1::inventory::ports::ZeusPart;
use crate::entities::{parts, part_catalog, part_conditions, products, new_part_forms, part_audit_log, work_orders};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

async fn assemble_part_entry(
    db: &sea_orm::DatabaseConnection,
    zeus_part: ZeusPart,
) -> Result<PartWithRelations, AppError> {
    let catalog_definition = part_catalog::Entity::find_by_id(zeus_part.part_catalog_id)
        .one(db)
        .await?
        .unwrap_or_else(|| part_catalog::Model {
            id: zeus_part.part_catalog_id,
            part_number: "UNKNOWN".to_string(),
            part_types_id: 1,
            mfg_number: "UNKNOWN".to_string(),
            description: None,
            part_mfg_status: 1,
            created_at: zeus_part.created_at,
            updated_at: zeus_part.updated_at,
            deleted_at: None,
        });

    let physical_condition = part_conditions::Entity::find_by_id(zeus_part.part_condition_id)
        .one(db)
        .await?
        .unwrap_or_else(|| part_conditions::Model {
            id: zeus_part.part_condition_id,
            name: "UNKNOWN".to_string(),
        });

    let mut installed_product = None;
    let mut customer_id = None;
    if let Some(prod_id) = zeus_part.product_id {
        if let Ok(Some(prod)) = products::Entity::find_by_id(prod_id).one(db).await {
            customer_id = Some(prod.customer_id);
            installed_product = Some(prod);
        }
    }

    let form = new_part_forms::Entity::find()
        .filter(new_part_forms::Column::SerialNumber.eq(&zeus_part.serial_number))
        .one(db)
        .await?;

    let mut approval_status = "approved".to_string();
    let mut denial_reason = None;
    let mut technician_id = None;

    if let Some(f) = form {
        let audit_log = part_audit_log::Entity::find()
            .filter(part_audit_log::Column::NewPartFormId.eq(f.id))
            .one(db)
            .await?;
        if let Some(log) = audit_log {
            approval_status = log.action.clone();
            denial_reason = log.reason.clone();
        } else {
            approval_status = "pending".to_string();
        }
        if let Ok(Some(wo)) = work_orders::Entity::find_by_id(f.work_order_id).one(db).await {
            technician_id = wo.technician_id;
        }
    }

    Ok(PartWithRelations {
        part_record: parts::Model {
            id: zeus_part.id,
            part_catalog_id: zeus_part.part_catalog_id,
            product_id: zeus_part.product_id,
            serial_number: zeus_part.serial_number,
            part_condition_id: zeus_part.part_condition_id,
            manufactured_date: zeus_part.manufactured_date,
            installation_date: zeus_part.installation_date,
            removal_date: zeus_part.removal_date,
            scrapped_date: zeus_part.scrapped_date,
            created_at: zeus_part.created_at,
            updated_at: zeus_part.updated_at,
            deleted_at: None,
        },
        catalog_definition,
        physical_condition,
        installed_product,
        approval_status,
        denial_reason,
        customer_id,
        technician_id,
    })
}

/// Retrieve detailed information for a single part.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/parts/{id}",
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the part")
    ),
    responses(
        (status = 200, description = "Detailed part information retrieved successfully", body = ApiResponse<PartDetailResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Part not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_part(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<PartDetailResponse>>, AppError> {
    let zeus_part = state.zeus_client.get_part(id).await?;
    let part_relation_data = assemble_part_entry(&state.db, zeus_part).await?;

    let detail = get_part::get_part_detail(&part_relation_data, &auth.role.name, auth.user.id)?;

    Ok(Json(ApiResponse::success(
        200,
        "Part retrieved successfully",
        detail,
    )))
}

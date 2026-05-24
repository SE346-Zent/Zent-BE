use axum::{extract::{State, Query}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::part_list_item::PartListItem;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::list_parts::{self, PartEntry};
use crate::entities::{parts, part_catalog, part_conditions, products, new_part_forms, part_audit_log, work_orders};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

pub async fn assemble_part_entries(
    db: &sea_orm::DatabaseConnection,
    zeus_parts: Vec<crate::services::v1::inventory::ports::ZeusPart>,
) -> Result<Vec<PartEntry>, AppError> {
    let mut entries = Vec::new();
    for zeus_part in zeus_parts {
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

        entries.push(PartEntry {
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
        });
    }
    Ok(entries)
}

/// Retrieve a filtered, paginated list of parts.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/parts",
    params(ListPartsQuery),
    responses(
        (status = 200, description = "List of parts retrieved successfully", body = ApiResponse<Vec<PartListItem>>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_parts(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListPartsQuery>,
) -> Result<Json<ApiResponse<Vec<PartListItem>>>, AppError> {
    let zeus_query = ListPartsQuery {
        model_code: query.model_code.clone(),
        part_type_id: query.part_type_id,
        approval_status: None, // Filter in memory via CanUserSeePart visibility check
        search: query.search.clone(),
        page: None,
        limit: Some(10000), // Fetch a large batch to filter/paginate in memory
        sort_by: None,
        sort_order: None,
    };

    let zeus_list = state.zeus_client.list_parts(&zeus_query).await?;
    let assembled = assemble_part_entries(&state.db, zeus_list.items).await?;
    
    let (items, meta) = list_parts::list_parts(&assembled, &auth.role.name, auth.user.id, &query);
    
    Ok(Json(ApiResponse::success_with_meta(
        200,
        "Parts retrieved successfully",
        items,
        meta,
    )))
}

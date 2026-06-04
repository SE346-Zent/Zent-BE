use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, Order};
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::history_response::{WorkOrderHistoryDetail, PartChangeEntry};
use crate::entities::{
    work_orders as work_orders_ent, work_order_state_history, users, work_order_closing_forms,
    work_order_image_links, images, part_changes, parts, part_catalog,
};

/// Retrieve the full state transition history for a specific work order.
///
/// - Admin/SuperAdmin: state history + closing form + rating + part changes + evidence photos.
/// - Technician: closing form + rating + part changes + evidence photos (no state history).
/// - Customer: technician name, current status, and ended_at (if completed).

#[utoipa::path(
    get, path = "/api/v1/work_orders/{id}/history",
    responses(
        (status = 200, description = "Work order history detail", body = ApiResponse<WorkOrderHistoryDetail>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Work order not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn history(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<WorkOrderHistoryDetail>>, AppError> {
    let wo = work_orders_ent::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let role_name = auth.role.name.as_str();

    match role_name {
        // --- Customer: simplified view ---
        "Customer" => {
            if wo.customer_id != auth.user.id {
                return Err(AppError::Forbidden("You do not have access to this work order".to_string()));
            }

            let status_name = luts.work_order_statuses
                .get(&wo.work_order_status_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let technician_name = if let Some(tech_id) = wo.technician_id {
                users::Entity::find_by_id(tech_id)
                    .one(db.as_ref())
                    .await?
                    .map(|u| u.full_name)
            } else {
                None
            };

            // ended_at only when the work order is completed (has a closing form)
            let ended_at = if wo.complete_form_id.is_some() {
                work_order_closing_forms::Entity::find()
                    .filter(work_order_closing_forms::Column::WorkOrderId.eq(wo.id))
                    .one(db.as_ref())
                    .await?
                    .map(|cf| crate::utils::time::to_utc7_string(cf.created_at))
            } else {
                None
            };

            let detail = crate::services::v1::work_orders::history::decide_customer_history(
                &wo, status_name, technician_name, ended_at,
            );
            Ok(Json(ApiResponse::success(200, "Work order history retrieved successfully", detail)))
        }

        // --- Technician / Admin / SuperAdmin ---
        "Technician" | "Admin" | "SuperAdmin" => {
            // Permission checks for non-superadmin roles
            match role_name {
                "Admin" => {
                    let Some(ref admin_province) = auth.user.province else {
                        return Err(AppError::Forbidden("Your admin profile does not have a province assigned".to_string()));
                    };
                    if admin_province != &wo.province {
                        return Err(AppError::Forbidden("You do not have permission to view work orders in this province".to_string()));
                    }
                }
                "Technician" => {
                    if wo.technician_id != Some(auth.user.id) {
                        return Err(AppError::Forbidden(format!("You are not assigned to work order {}", wo.work_order_number)));
                    }
                }
                _ => {}
            }

            let include_state_history = role_name == "Admin" || role_name == "SuperAdmin";

            // State history (only for Admin/SuperAdmin)
            let history_rows: Vec<(work_order_state_history::Model, Option<users::Model>)> = if include_state_history {
                work_order_state_history::Entity::find()
                    .filter(work_order_state_history::Column::WorkOrderId.eq(id))
                    .order_by(work_order_state_history::Column::ChangedAt, Order::Asc)
                    .find_also_related(users::Entity)
                    .all(db.as_ref())
                    .await?
            } else {
                vec![]
            };

            // Closing form
            let closing_form = if let Some(cf_id) = wo.complete_form_id {
                work_order_closing_forms::Entity::find_by_id(cf_id).one(db.as_ref()).await?
            } else {
                None
            };

            // Rating
            let rating = crate::entities::work_order_ratings::Entity::find()
                .filter(crate::entities::work_order_ratings::Column::WorkOrderId.eq(id))
                .one(db.as_ref())
                .await?;

            // Part changes (via closing form)
            let part_changes_entries = if let Some(ref cf) = closing_form {
                let pc_rows: Vec<(part_changes::Model, Option<parts::Model>)> = part_changes::Entity::find()
                    .filter(part_changes::Column::WorkOrderClosingFormId.eq(cf.id))
                    .find_also_related(parts::Entity)
                    .all(db.as_ref())
                    .await?;

                // Batch-fetch part catalogs to avoid N+1
                let catalog_ids: Vec<Uuid> = pc_rows.iter()
                    .filter_map(|(_, part_opt)| part_opt.as_ref().map(|p| p.part_catalog_id))
                    .collect();
                let catalog_map: std::collections::HashMap<Uuid, String> = if catalog_ids.is_empty() {
                    std::collections::HashMap::new()
                } else {
                    part_catalog::Entity::find()
                        .filter(part_catalog::Column::Id.is_in(catalog_ids))
                        .all(db.as_ref())
                        .await?
                        .into_iter()
                        .map(|c| (c.id, c.part_number))
                        .collect()
                };

                let mut entries = Vec::with_capacity(pc_rows.len());
                for (pc, part_opt) in pc_rows {
                    let (part_number, serial_number) = if let Some(ref part) = part_opt {
                        let pn = catalog_map.get(&part.part_catalog_id)
                            .cloned()
                            .unwrap_or_else(|| "Unknown".to_string());
                        (pn, part.serial_number.clone())
                    } else {
                        ("Unknown".to_string(), "Unknown".to_string())
                    };
                    entries.push(PartChangeEntry {
                        part_id: pc.part_id,
                        part_number,
                        serial_number,
                        change_type: pc.change_type,
                        created_at: crate::utils::time::to_utc7_string(pc.created_at),
                    });
                }
                entries
            } else {
                vec![]
            };

            // Evidence photos (from work_order_image_links)
            let evidence_photos: Vec<String> = work_order_image_links::Entity::find()
                .filter(work_order_image_links::Column::WorkOrderId.eq(id))
                .find_also_related(images::Entity)
                .all(db.as_ref())
                .await?
                .into_iter()
                .filter_map(|(_, img)| img)
                .map(|img| img.object_name)
                .collect();

            let detail = crate::services::v1::work_orders::history::decide_get_history_detail(
                history_rows, &luts, wo, closing_form, rating, part_changes_entries, evidence_photos, include_state_history,
            );
            Ok(Json(ApiResponse::success(200, "Work order history retrieved successfully", detail)))
        }

        _ => Err(AppError::Forbidden("Your role is not permitted to access this resource".to_string())),
    }
}

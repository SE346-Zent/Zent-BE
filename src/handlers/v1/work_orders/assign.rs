use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait, Set};
use uuid::Uuid;
use validator::Validate;
use sea_orm::{QueryFilter, ColumnTrait};
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::assign_request::AssignWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::{work_orders as work_orders_ent, users, chat_rooms, chat_room_members};

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/assign", request_body = AssignWorkOrderRequest,
    responses(
        (status = 200, description = "Work order assigned successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn assign(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(mongodb): State<Arc<mongodb::Database>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_opt): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AssignWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    // Write-through: use the cache for individual work order instead of querying DB
    let work_order = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    if auth.role.name == "Admin" {
        let p = auth.user.province.as_ref().ok_or_else(|| AppError::Forbidden("Admin has no province assigned".to_string()))?;
        if p != &work_order.province { return Err(AppError::Forbidden("Admin province does not match work order province".to_string())); }
    }

    let technician_work_orders = work_orders_ent::Entity::find().filter(work_orders_ent::Column::TechnicianId.eq(payload.technician_id)).all(db.as_ref()).await?;
    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned").ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Assigned' status missing")))?;
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed").ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    let effect = crate::services::v1::work_orders::assign::decide_assign_work_order(payload.clone(), work_order.clone(), technician_work_orders, &luts.policies, assigned_status_id, done_status_id, auth.user.id)?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move { effect.work_order.update(txn).await?; effect.state_history.insert(txn).await?; Ok(()) }))
        .await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(db.as_ref(), valkey_client.clone(), luts.as_ref(), id).await;

    let tech = users::Entity::find_by_id(payload.technician_id).one(db.as_ref()).await?;
    let cust = users::Entity::find_by_id(work_order.customer_id).one(db.as_ref()).await?;

    // Send push + in-app notifications to both technician and customer
    if let (Some(ref t), Some(ref c)) = (&tech, &cust) {
        let notification_data = serde_json::json!({
            "workOrderId": work_order.id,
            "workOrderNumber": work_order.work_order_number,
            "technicianName": t.full_name,
            "appointment": work_order.appointment.to_string(),
        });

        // Notify the technician
        let _ = crate::services::v1::notifications::send_notification::send_notification(
            mongodb.as_ref(),
            valkey_client.clone(),
            db.as_ref(),
            t.id,
            "work_order_assigned",
            "New Work Order Assigned",
            &format!("You have been assigned to work order {}", work_order.work_order_number),
            notification_data.clone(),
        ).await;

        // Notify the customer
        let _ = crate::services::v1::notifications::send_notification::send_notification(
            mongodb.as_ref(),
            valkey_client.clone(),
            db.as_ref(),
            c.id,
            "work_order_assigned",
            "Work Order Assigned",
            &format!("Your work order {} has been assigned to technician {}", work_order.work_order_number, t.full_name),
            notification_data,
        ).await;
    }

    // Send email notification (existing)
    if let Some(rmq) = rabbitmq_opt.as_ref() {
        if let (Some(ref t), Some(ref c)) = (&tech, &cust) {
            let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(rmq, &templates, &c.email, &c.full_name, &work_order.work_order_number, &t.full_name, &work_order.appointment.to_string()).await;
        }
    }
    // Auto-create 1-on-1 chat room between technician and customer
    if let (Some(ref t), Some(ref c)) = (&tech, &cust) {
        let _ = ensure_chat_room(db.as_ref(), t.id, c.id, work_order.id).await;
    }

    Ok(Json(ApiResponse::success(200, "Work order assigned successfully", ())))
}

/// Ensure a 1-on-1 chat room exists between two users. Creates one if not found.
async fn ensure_chat_room(
    db: &DatabaseConnection,
    user_a: Uuid,
    user_b: Uuid,
    work_order_id: Uuid,
) -> Result<Uuid, AppError> {
    // Find rooms where user_a is a member
    let a_rooms: Vec<Uuid> = chat_room_members::Entity::find()
        .filter(chat_room_members::Column::UserId.eq(user_a))
        .all(db)
        .await?
        .into_iter()
        .map(|m| m.room_id)
        .collect();

    // Check if any of those rooms also have user_b
    if !a_rooms.is_empty() {
        let shared = chat_room_members::Entity::find()
            .filter(chat_room_members::Column::RoomId.is_in(a_rooms))
            .filter(chat_room_members::Column::UserId.eq(user_b))
            .one(db)
            .await?;
        if let Some(m) = shared {
            return Ok(m.room_id);
        }
    }

    // Create new room
    let now = chrono::Utc::now();
    let room_id = Uuid::new_v4();

    let room = chat_rooms::ActiveModel {
        id: Set(room_id),
        room_name: Set("Direct Chat".to_string()),
        created_by: Set(user_a),
        work_order_id: Set(Some(work_order_id)),
        created_at: Set(now),
        updated_at: Set(None),
        deleted_at: Set(None),
    };
    room.insert(db).await?;

    let m1 = chat_room_members::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(user_a),
        created_at: Set(now),
        updated_at: Set(None),
        deleted_at: Set(None),
    };
    m1.insert(db).await?;

    let m2 = chat_room_members::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(user_b),
        created_at: Set(now),
        updated_at: Set(None),
        deleted_at: Set(None),
    };
    m2.insert(db).await?;

    Ok(room_id)
}

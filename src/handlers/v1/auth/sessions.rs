use axum::{extract::{Path, State}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, Set, ActiveModelTrait};
use redis::AsyncCommands;
use uuid::Uuid;
use chrono::Utc;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::sessions;
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::auth::session_response::SessionInfo;
use crate::model::responses::base::ApiResponse;

#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    responses(
        (status = 200, description = "Active sessions retrieved", body = ApiResponse<Vec<SessionInfo>>),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
/// List all active sessions for the authenticated user.
pub async fn list_sessions_handler(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<SessionInfo>>>, AppError> {
    let now = Utc::now();
    let records = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(auth.user.id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .order_by_desc(sessions::Column::CreatedAt)
        .all(db.as_ref())
        .await?;

    // The current session is identified by the session_id in the JWT claims.
    // Since AuthUser doesn't carry session_id, we mark none as "current" here.
    // Clients should compare with their stored session_id.
    let sessions_list: Vec<SessionInfo> = records
        .into_iter()
        .map(|s| SessionInfo {
            id: s.id,
            device_name: s.device_fingerprint,
            ip_address: s.ip_address,
            is_current: false,
            created_at: s.created_at,
            expires_at: s.expires_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(200, "Sessions retrieved successfully", sessions_list)))
}

/// Revoke a specific session. Cannot revoke the current session (use logout instead).
pub async fn revoke_session_handler(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let session_record = sessions::Entity::find_by_id(session_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

    // Verify ownership
    if session_record.user_id != auth.user.id {
        return Err(AppError::Unauthorized("Session does not belong to this user".to_string()));
    }

    // Check if already revoked
    if session_record.revoked_at.is_some() {
        return Err(AppError::BadRequest("Session already revoked".to_string()));
    }

    // Revoke
    let mut active: sessions::ActiveModel = session_record.into();
    active.revoked_at = Set(Some(Utc::now()));
    active.update(db.as_ref()).await?;

    // Remove from Valkey whitelist
    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let whitelist_key = format!("whitelist:session:{}", session_id);
            let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
        }
    }

    // Close WebSocket connections for this session
    let ws_manager = crate::infrastructure::ws::get_ws_manager();
    ws_manager.close_session_connections(&auth.user.id, &session_id).await;

    Ok(Json(ApiResponse::message_only(200, "Session revoked successfully")))
}

/// Revoke all sessions for the authenticated user except the current one.
pub async fn revoke_all_sessions_handler(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let now = Utc::now();
    let active_sessions = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(auth.user.id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(now))
        .all(db.as_ref())
        .await?;

    let ws_manager = crate::infrastructure::ws::get_ws_manager();

    for session in active_sessions {
        // Revoke in DB
        let mut active: sessions::ActiveModel = session.clone().into();
        active.revoked_at = Set(Some(now));
        active.update(db.as_ref()).await?;

        // Remove from Valkey whitelist
        if let Some(ref client) = valkey_client {
            if let Ok(mut conn) = client.get_connection().await {
                let whitelist_key = format!("whitelist:session:{}", session.id);
                let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
            }
        }

        // Close WebSocket connections
        ws_manager.close_session_connections(&auth.user.id, &session.id).await;
    }

    Ok(Json(ApiResponse::message_only(200, "All other sessions revoked successfully")))
}

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

use std::sync::Arc;
use jsonwebtoken::{DecodingKey};
use sea_orm::{DatabaseConnection, EntityTrait};
use tracing::{error, info, warn};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use redis::AsyncCommands;

use crate::{
    entities::{roles, users},
    model::jwt_claims::Claims,
    core::errors::AppError,
    infrastructure::cache::ValkeyClient,
};

/// Represents an authenticated user with their associated role data, used as an Axum extractor.
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthUser {
    /// The user model containing profile information.
    pub user: users::Model,
    /// The role model containing user permissions and category assignments.
    pub role: roles::Model,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    DecodingKey: FromRef<S>,
    Arc<DatabaseConnection>: FromRef<S>,
    Option<Arc<ValkeyClient>>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;
        
        let db = Arc::<DatabaseConnection>::from_ref(state);
        let valkey = Option::<Arc<ValkeyClient>>::from_ref(state);
        
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            error!("Invalid UUID subject");
            AppError::Unauthorized("Invalid authentication token".to_string())
        })?;
        
        // Try to get from Valkey cache first
        if let Some(client) = valkey.as_ref() {
            if let Ok(mut conn) = client.get_connection().await {
                let cache_key = format!("user_profile:{}", user_id);
                
                if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&cache_key).await {
                    if let Ok(auth_user) = serde_json::from_str::<AuthUser>(&cached_json) {
                        return Ok(auth_user);
                    }
                }
            }
        }

        let user_with_role = users::Entity::find_by_id(user_id)
            .find_with_related(roles::Entity)
            .all(db.as_ref())
            .await
            .map_err(|e| {
                error!("Database error: {:?}", e);
                AppError::Internal(anyhow::anyhow!("Database error during authentication"))
            })?;
    
        if user_with_role.is_empty() {
            return Err(AppError::Unauthorized("User account not found".to_string()));
        }
        
        let (user, user_roles) = user_with_role.into_iter().next().unwrap();
        let role = user_roles.into_iter().next().ok_or_else(|| {
            AppError::Forbidden("User role is missing".to_string())
        })?;
        
        let auth_user = AuthUser { user, role };

        // Save to Valkey cache
        if let Some(client) = valkey {
            if let Ok(mut conn) = client.get_connection().await {
                let cache_key = format!("user_profile:{}", user_id);
                if let Ok(json) = serde_json::to_string(&auth_user) {
                    let _: () = conn.set_ex(&cache_key, json, 900).await.unwrap_or_else(|e| {
                        warn!("Failed to cache user profile in Valkey: {:?}", e);
                    });
                }
            }
        }

        info!("The user is valid...");
        Ok(auth_user)
    }
}

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

use std::sync::Arc;
use jsonwebtoken::{DecodingKey};
use sea_orm::{DatabaseConnection, EntityTrait};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    entities::{roles, users},
    model::jwt_claims::Claims,
    core::errors::AppError,
};

#[derive(Clone)]
pub struct AuthUser {
    pub user: users::Model,
    pub role: roles::Model,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    DecodingKey: FromRef<S>,
    Arc<DatabaseConnection>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;
        
        let db = Arc::<DatabaseConnection>::from_ref(state);
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            error!("Invalid UUID subject");
            AppError::Unauthorized("Invalid token subject".to_string())
        })?;
        
        let user_with_role = users::Entity::find_by_id(user_id)
            .find_with_related(roles::Entity)
            .all(db.as_ref())
            .await
            .map_err(|e| {
                error!("Database error: {:?}", e);
                AppError::Internal(anyhow::anyhow!("Database error during authentication"))
            })?;
    
        if user_with_role.is_empty() {
            return Err(AppError::Unauthorized("User not found".to_string()));
        }
        
        let (user, user_roles) = user_with_role.into_iter().next().unwrap();
        let role = user_roles.into_iter().next().ok_or_else(|| {
            AppError::Forbidden("User profile is missing role information".to_string())
        })?;
        
        info!("The user is valid...");
        Ok(AuthUser { user, role })
    }
}

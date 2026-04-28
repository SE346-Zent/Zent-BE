use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

use jsonwebtoken::{DecodingKey};
use sea_orm::{DatabaseConnection, EntityTrait};
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    entities::{roles, users},
    model::jwt_claims::Claims,
    extractor::jwt_claims::AuthError,
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
    DatabaseConnection: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::from_request_parts(parts, state).await?;
        
        let db = DatabaseConnection::from_ref(state);
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            error!("Invalid UUID subject");
            AuthError::InvalidTokenError
        })?;
        
        let user_with_role = users::Entity::find_by_id(user_id)
            .find_with_related(roles::Entity)
            .all(&db)
            .await
            .map_err(|_| AuthError::InternalServerError)?;
    
        if user_with_role.is_empty() {
            return Err(AuthError::InvalidTokenError);
        }
        
        let (user, roles) = user_with_role.into_iter().next().unwrap();
        let role = roles.into_iter().next().ok_or(AuthError::InvalidTokenError)?;
        
        info!("The user is valid...");
        Ok(AuthUser { user, role })
    }
}

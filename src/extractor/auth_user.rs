use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use sea_orm::{DatabaseConnection, EntityTrait};
use tracing::error;
use uuid::Uuid;

use crate::{
    entities::{role, user},
    model::auth::jwt_claims::Claims,
};
use super::jwt_claims::AuthError;

pub struct AuthUser {
    pub user: user::Model,
    pub role: role::Model,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    DecodingKey: FromRef<S>,
    DatabaseConnection: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|e| {
                    error!("Auth extract error: {}", e);
                    AuthError::AuthHeaderNotFound
                })?;

        let decoding_key = DecodingKey::from_ref(state);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 10;
        validation.set_required_spec_claims(&["exp", "sub", "iat"]);

        let token_data = decode::<Claims>(bearer.token(), &decoding_key, &validation).map_err(
            |e| {
                error!("Token decode error: {}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        AuthError::ExpiredTokenError
                    }
                    _ => AuthError::InvalidTokenError,
                }
            },
        )?;

        let db = DatabaseConnection::from_ref(state);
        let user_id = Uuid::parse_str(&token_data.claims.sub).map_err(|_| {
            error!("Invalid UUID subject");
            AuthError::InvalidTokenError
        })?;

        // Utilization of find_with_related mapping Role context iteratively
        let user_with_role = user::Entity::find_by_id(user_id)
            .find_with_related(role::Entity)
            .all(&db)
            .await
            .map_err(|_| AuthError::InternalServerError)?;

        if user_with_role.is_empty() {
            return Err(AuthError::InvalidTokenError);
        }

        let (user, roles) = user_with_role.into_iter().next().unwrap();
        let role = roles.into_iter().next().ok_or(AuthError::InvalidTokenError)?;

        Ok(AuthUser { user, role })
    }
}

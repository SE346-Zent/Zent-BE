use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use tracing::error;

use crate::model::jwt_claims::Claims;

pub enum AuthError {
    InvalidTokenError,
    ExpiredTokenError,
    AuthHeaderNotFound,
    InternalServerError,
    MissingRole,
    Forbidden,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::InvalidTokenError => StatusCode::UNAUTHORIZED,
            AuthError::ExpiredTokenError => StatusCode::UNAUTHORIZED,
            AuthError::AuthHeaderNotFound => StatusCode::UNAUTHORIZED,
            AuthError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            AuthError::MissingRole => StatusCode::FORBIDDEN,
            AuthError::Forbidden => StatusCode::FORBIDDEN,
        };
        status.into_response()
    }
}

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    DecodingKey: FromRef<S>,
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

        let token_data = decode::<Claims>(bearer.token(), &decoding_key, &validation)
            .map_err(|e| {
                error!("Token decode error: {}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredTokenError,
                    _ => AuthError::InvalidTokenError,
                }
            })?;

        Ok(token_data.claims)
    }
}

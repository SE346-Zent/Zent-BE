use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use tracing::{error, warn};

use crate::core::errors::AppError;
use crate::model::jwt_claims::Claims;

impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
    DecodingKey: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|e| {
                    warn!(
                        message = "Authorization header not found",
                        error.message = %e,
                        error.details = ?e,
                        "Authorization header extraction failed"
                    );
                    AppError::Unauthorized("Authorization header not found".to_string())
                })?;

        let decoding_key = DecodingKey::from_ref(state);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 10;
        validation.set_required_spec_claims(&["exp", "sub", "iat"]);

        let token_data =
            decode::<Claims>(bearer.token(), &decoding_key, &validation).map_err(|e| {
                error!(
                    message = "Invalid token",
                    error.message = %e,
                    error.details = ?e,
                    "JWT decoding or validation failed"
                );
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        AppError::Unauthorized("Token has expired".to_string())
                    }
                    _ => AppError::Unauthorized("Invalid token".to_string()),
                }
            })?;

        Ok(token_data.claims)
    }
}

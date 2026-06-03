use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Centralized application error type.
pub enum AppError {
    /// 400 Bad Request: The request is malformed or invalid.
    BadRequest(String),
    /// 401 Unauthorized: Authentication is required or failed.
    Unauthorized(String),
    /// 403 Forbidden: The authenticated user lacks permission.
    Forbidden(String),
    /// 404 Not Found: The requested resource does not exist.
    NotFound(String),
    /// 409 Conflict: The request conflicts with the current state (e.g., duplicate email).
    Conflict(String),
    /// 422 Unprocessable Entity: Request validation failed.
    ValidationError(String),
    /// 503 Service Unavailable: A required dependency (e.g., DB, MQ) is unavailable.
    ServiceUnavailable(String),
    /// 500 Internal Server Error: An unexpected error occurred.
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => {
                tracing::warn!(
                    code = %StatusCode::BAD_REQUEST.as_u16(),
                    error.message = "BadRequest",
                    error.details = %msg,
                    "HTTP Bad Request response dispatched"
                );
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::Unauthorized(msg) => {
                tracing::warn!(
                    code = %StatusCode::UNAUTHORIZED.as_u16(),
                    error.message = "Unauthorized",
                    error.details = %msg,
                    "HTTP Unauthorized response dispatched"
                );
                (StatusCode::UNAUTHORIZED, msg)
            }
            AppError::Forbidden(msg) => {
                tracing::warn!(
                    code = %StatusCode::FORBIDDEN.as_u16(),
                    error.message = "Forbidden",
                    error.details = %msg,
                    "HTTP Forbidden response dispatched"
                );
                (StatusCode::FORBIDDEN, msg)
            }
            AppError::NotFound(msg) => {
                tracing::warn!(
                    code = %StatusCode::NOT_FOUND.as_u16(),
                    error.message = "NotFound",
                    error.details = %msg,
                    "HTTP Not Found response dispatched"
                );
                (StatusCode::NOT_FOUND, msg)
            }
            AppError::Conflict(msg) => {
                tracing::warn!(
                    code = %StatusCode::CONFLICT.as_u16(),
                    error.message = "Conflict",
                    error.details = %msg,
                    "HTTP Conflict response dispatched"
                );
                (StatusCode::CONFLICT, msg)
            }
            AppError::ValidationError(msg) => {
                tracing::warn!(
                    code = %StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
                    error.message = "ValidationError",
                    error.details = %msg,
                    "HTTP Validation Error response dispatched"
                );
                (StatusCode::UNPROCESSABLE_ENTITY, msg)
            }
            AppError::ServiceUnavailable(msg) => {
                tracing::error!(
                    code = %StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                    error.message = "ServiceUnavailable",
                    error.details = %msg,
                    "HTTP Service Unavailable response dispatched"
                );
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
            AppError::Internal(err) => {
                tracing::error!(
                    code = %StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    error.message = "InternalError",
                    error.details = %err,
                    error.details_debug = ?err,
                    "HTTP Internal Server Error response dispatched"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "statusCode": status.as_u16(),
            "message": error_message,
            "data": null,
            "meta": null,
        }));

        (status, body).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            AppError::ServiceUnavailable(msg) => write!(f, "Service Unavailable: {}", msg),
            AppError::Internal(err) => write!(f, "Internal Error: {:?}", err),
        }
    }
}

impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Structured error response schema for OpenAPI (Utoipa) documentation.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    #[schema(example = 400)]
    pub status_code: u16,
    #[schema(example = "Generic error mapping format instance")]
    pub message: String,
    #[schema(value_type = Option<Object>)]
    pub data: Option<serde_json::Value>,
    #[schema(value_type = Option<Object>)]
    pub meta: Option<serde_json::Value>,
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::Internal(anyhow::Error::new(err).context("Database operation failed"))
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Internal(anyhow::Error::new(err).context("Cache operation failed"))
    }
}

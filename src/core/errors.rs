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
    /// 422 Unprocessable Entity: The requested change is not allowed by a business rule
    /// (e.g., the new product is not covered by an active warranty). The error message
    /// is phrased so the front-end can surface it to the end user.
    WarrantyError(String),
    /// 503 Service Unavailable: A required dependency (e.g., DB, MQ) is unavailable.
    ServiceUnavailable(String),
    /// 500 Internal Server Error: An unexpected error occurred.
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_type) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, "bad_request"),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg, "unauthorized"),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg, "forbidden"),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg, "not_found"),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg, "conflict"),
            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg, "validation"),
            AppError::WarrantyError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg, "warranty"),
            AppError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg, "service_unavailable"),
            AppError::Internal(err) => {
                tracing::error!(
                    error.message = %err,
                    error.details = ?err,
                    "Internal server error occurred"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                    "internal",
                )
            }
        };

        // Track error metric (safe: metrics are initialized at startup)
        crate::infrastructure::metrics::init().app_error_total.add(1, &[
            opentelemetry::KeyValue::new("type", error_type),
            opentelemetry::KeyValue::new("status", status.as_u16().to_string()),
        ]);

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
            AppError::WarrantyError(msg) => write!(f, "Warranty Error: {}", msg),
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
        AppError::Internal(anyhow::anyhow!("Database error: {}", err))
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Internal(anyhow::anyhow!("Cache error: {}", err))
    }
}


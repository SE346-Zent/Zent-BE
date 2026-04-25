use serde::{Deserialize, Serialize};

use super::pagination::PaginationResponse;

/// Generic API response envelope.
///
/// Every successful response follows this shape:
/// ```json
/// {
///   "statusCode": 200,
///   "message": "Success",
///   "data": { ... },
///   "meta": { "currentPage": 1, "limit": 20, ... }   // optional
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T: Serialize> {
    /// HTTP status code indicator.
    #[schema(example = 200)]
    pub status_code: u16,

    /// Human-readable message describing the result.
    #[schema(example = "Success")]
    pub message: String,

    /// Response payload (domain-specific data).
    pub data: Option<T>,

    /// Optional pagination metadata.
    pub meta: Option<PaginationResponse>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Build a successful response without pagination metadata.
    pub fn success(status_code: u16, message: impl Into<String>, data: T) -> Self {
        Self {
            status_code,
            message: message.into(),
            data: Some(data),
            meta: None,
        }
    }

    /// Build a successful response with pagination metadata.
    pub fn success_with_meta(
        status_code: u16,
        message: impl Into<String>,
        data: T,
        meta: PaginationResponse,
    ) -> Self {
        Self {
            status_code,
            message: message.into(),
            data: Some(data),
            meta: Some(meta),
        }
    }

}

impl ApiResponse<()> {
    /// Build a message-only response (no data payload).
    pub fn message_only(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            status_code,
            message: message.into(),
            data: None,
            meta: None,
        }
    }
}

/// A non-generic version of ApiResponse for message-only endpoints.
/// Used in utoipa schema generation where `ApiResponse<()>` would panic.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageOnlyResponse {
    #[schema(example = 201)]
    pub status_code: u16,
    #[schema(example = "Operation successful")]
    pub message: String,
}


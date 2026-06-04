use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

/// Request payload for creating a warranty for a product.
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWarrantyRequest {
    /// Serial number of the product in SCM.
    #[validate(length(min = 1, max = 255))]
    pub serial_number: String,

    /// Warranty start date (ISO 8601, e.g. "2025-01-15T00:00:00Z").
    pub start_date: chrono::DateTime<chrono::Utc>,

    /// Warranty end date (ISO 8601, e.g. "2027-01-15T00:00:00Z").
    pub end_date: chrono::DateTime<chrono::Utc>,
}

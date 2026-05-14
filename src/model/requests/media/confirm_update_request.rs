use utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmUpdateRequest {
    #[validate(length(min = 1))]
    pub unique_file_name: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Client-side Unix timestamp when the update was initiated.
    /// Validated against server time using the `internet_time_drift_minutes` policy.
    pub internet_time: i64,
}

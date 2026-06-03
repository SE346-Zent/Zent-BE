use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianMetricsResponse {
    /// Number of work orders currently assigned to the technician (not closed).
    pub active_jobs: i64,
    /// Average rating across all rated work orders (0.0 if no ratings).
    pub overall_rating: f64,
}

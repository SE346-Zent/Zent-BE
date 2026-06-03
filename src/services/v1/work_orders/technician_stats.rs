use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicianStatsInput {
    pub total_work_orders: i64,
    pub active_jobs: i64,
    pub rating_sum: i64,
    pub rating_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicianStatsSnapshot {
    pub total_work_orders: i64,
    pub active_jobs: i64,
    pub rating_sum: i64,
    pub rating_count: i64,
}

impl TechnicianStatsSnapshot {
    /// Average rating across all rated work orders (used by admin analytics).
    pub fn average_rating(&self) -> f64 {
        if self.rating_count == 0 {
            0.0
        } else {
            (self.rating_sum as f64 / self.rating_count as f64 * 100.0).round() / 100.0
        }
    }
}

pub fn decide_technician_stats(input: TechnicianStatsInput) -> TechnicianStatsSnapshot {
    TechnicianStatsSnapshot {
        total_work_orders: input.total_work_orders,
        active_jobs: input.active_jobs,
        rating_sum: input.rating_sum,
        rating_count: input.rating_count,
    }
}
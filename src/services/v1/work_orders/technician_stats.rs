use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicianStatsInput {
    pub total_work_orders: i64,
    pub rating_sum: i64,
    pub rating_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicianStatsSnapshot {
    pub total_work_orders: i64,
    pub rating_sum: i64,
    pub rating_count: i64,
}

impl TechnicianStatsSnapshot {
    pub fn average_rating(&self) -> f64 {
        if self.total_work_orders == 0 {
            0.0
        } else {
            self.rating_sum as f64 / self.total_work_orders as f64
        }
    }
}

pub fn decide_technician_stats(input: TechnicianStatsInput) -> TechnicianStatsSnapshot {
    TechnicianStatsSnapshot {
        total_work_orders: input.total_work_orders,
        rating_sum: input.rating_sum,
        rating_count: input.rating_count,
    }
}
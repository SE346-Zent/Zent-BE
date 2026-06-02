use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdminAnalyticsResponse {
    pub total_orders: TotalMetric,
    pub total_imported_parts: TotalMetric,
    pub total_returned_parts: TotalMetric,
    pub job_completion_trend: JobCompletionTrend,
    pub part_categories: Vec<PartCategoryEntry>,
    pub technician_performance: Vec<TechnicianPerformanceEntry>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotalMetric {
    pub value: i64,
    pub percent_change: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobCompletionTrend {
    pub labels: Vec<String>,
    pub current: Vec<i64>,
    pub previous: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartCategoryEntry {
    pub category_name: String,
    pub count: i64,
    pub percent: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TechnicianPerformanceEntry {
    pub technician_id: uuid::Uuid,
    pub technician_name: String,
    pub total_work_orders: i64,
    pub rating_count: i64,
    pub average_rating: f64,
}

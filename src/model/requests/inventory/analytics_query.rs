use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AnalyticsQuery {
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_period() -> String {
    "7d".to_string()
}

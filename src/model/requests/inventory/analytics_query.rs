use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct AnalyticsQuery {
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "weekly".to_string()
}

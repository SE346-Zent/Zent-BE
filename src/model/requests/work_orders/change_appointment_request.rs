use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAppointmentRequest {
    /// New appointment datetime (ISO 8601)
    pub new_appointment: DateTime<Utc>,
}

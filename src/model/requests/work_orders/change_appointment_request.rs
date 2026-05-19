use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAppointmentRequest {
    /// New appointment datetime (ISO 8601).
    /// Conflict with other work orders of the same technician is checked
    /// in the handler, not here.
    pub new_appointment: DateTime<Utc>,
}

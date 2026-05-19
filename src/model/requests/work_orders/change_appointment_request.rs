use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeAppointmentRequest {
    /// New appointment datetime (ISO 8601). Must be in the future.
    #[validate(custom(function = "validate_future_date"))]
    pub new_appointment: DateTime<Utc>,
}

/// Rejects appointment datetimes that are not in the future.
fn validate_future_date(dt: &DateTime<Utc>) -> Result<(), validator::ValidationError> {
    if *dt <= Utc::now() {
        let mut err = validator::ValidationError::new("appointment_in_past");
        err.message = Some("Appointment must be in the future".into());
        return Err(err);
    }
    Ok(())
}

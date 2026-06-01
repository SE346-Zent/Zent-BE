use chrono::{DateTime, FixedOffset, Utc};

/// Retrieve the FixedOffset for UTC+7.
pub fn get_utc7_offset() -> FixedOffset {
    FixedOffset::east_opt(7 * 3600).unwrap()
}

/// Convert a UTC DateTime to a UTC+7 DateTime.
pub fn to_utc7_time(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&get_utc7_offset())
}

/// Convert a UTC DateTime to a GMT+7 formatted ISO 8601 string.
/// e.g. "2026-05-27T14:30:00+07:00"
pub fn to_utc7_string(dt: DateTime<Utc>) -> String {
    to_utc7_time(dt).to_rfc3339()
}

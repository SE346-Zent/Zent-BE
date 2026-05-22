use chrono::{DateTime, FixedOffset, Utc};

/// Retrieve the FixedOffset for UTC+7.
pub fn get_utc7_offset() -> FixedOffset {
    FixedOffset::east_opt(7 * 3600).unwrap()
}

/// Convert a UTC DateTime to a UTC+7 DateTime.
pub fn to_utc7_time(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&get_utc7_offset())
}

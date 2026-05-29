use chrono::{DateTime, Timelike, Utc};

pub(super) struct TimeParts {
    pub(super) date: String,
    pub(super) hour: u32,
}

pub(super) fn time_parts(timestamp_ms: i64) -> TimeParts {
    let timestamp =
        DateTime::<Utc>::from_timestamp_millis(timestamp_ms).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    TimeParts {
        date: timestamp.format("%Y-%m-%d").to_string(),
        hour: timestamp.hour(),
    }
}

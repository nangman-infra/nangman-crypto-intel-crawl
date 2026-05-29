use super::SourceHealthRecord;
use super::input::HealthRecordInput;
use super::status::{BackoffState, HealthStatus};
use crate::registry::Source;

impl SourceHealthRecord {
    pub(crate) fn ok(
        source: &Source,
        checked_at_ms: i64,
        observed_at_ms: i64,
        items_seen: usize,
        events_written: usize,
        duplicate_events_skipped: usize,
    ) -> Self {
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status: HealthStatus::from_items_seen(items_seen),
            items_seen,
            events_written,
            duplicate_events_skipped,
            latency_ms: observed_at_ms.saturating_sub(checked_at_ms),
            error: None,
            backoff_state: BackoffState::None,
        })
    }

    pub(crate) fn failed(
        source: &Source,
        checked_at_ms: i64,
        observed_at_ms: i64,
        error: String,
    ) -> Self {
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status: HealthStatus::Failed,
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: observed_at_ms.saturating_sub(checked_at_ms),
            error: Some(error),
            backoff_state: BackoffState::Scheduled,
        })
    }

    pub(crate) fn not_modified(source: &Source, checked_at_ms: i64, observed_at_ms: i64) -> Self {
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status: HealthStatus::NotModified,
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: observed_at_ms.saturating_sub(checked_at_ms),
            error: None,
            backoff_state: BackoffState::None,
        })
    }

    pub(crate) fn skipped_backoff(
        source: &Source,
        checked_at_ms: i64,
        observed_at_ms: i64,
        backoff_until_ms: Option<i64>,
    ) -> Self {
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status: HealthStatus::SkippedBackoff,
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: 0,
            error: backoff_until_ms.map(|until| format!("backoff_until_ms={until}")),
            backoff_state: BackoffState::Active,
        })
    }

    pub(crate) fn skipped_cadence(
        source: &Source,
        checked_at_ms: i64,
        observed_at_ms: i64,
        next_due_at_ms: i64,
    ) -> Self {
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status: HealthStatus::SkippedCadence,
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: 0,
            error: Some(format!("next_due_at_ms={next_due_at_ms}")),
            backoff_state: BackoffState::None,
        })
    }
}

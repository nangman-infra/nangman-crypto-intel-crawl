use super::status::{BackoffState, HealthStatus};
use crate::registry::Source;

pub(super) struct HealthRecordInput<'a> {
    pub(super) source: &'a Source,
    pub(super) checked_at_ms: i64,
    pub(super) observed_at_ms: i64,
    pub(super) status: HealthStatus,
    pub(super) items_seen: usize,
    pub(super) events_written: usize,
    pub(super) duplicate_events_skipped: usize,
    pub(super) latency_ms: i64,
    pub(super) error: Option<String>,
    pub(super) backoff_state: BackoffState,
}

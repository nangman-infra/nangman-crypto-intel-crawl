use super::hash::short_hash;
use serde::Serialize;

mod constructors;
mod input;
mod status;

use input::HealthRecordInput;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceHealthRecord {
    pub(super) schema_version: String,
    pub(super) health_event_id: String,
    pub(super) source_id: String,
    pub(super) source_category: String,
    pub(super) fetch_method: String,
    pub(super) adapter: Option<String>,
    pub(super) checked_at_ms: i64,
    pub(super) observed_at_ms: i64,
    pub(super) fetch_status: String,
    pub(super) http_status_or_error: Option<String>,
    pub(super) latency_ms: i64,
    pub(super) items_fetched: usize,
    pub(super) items_emitted: usize,
    pub(super) dedup_dropped: usize,
    pub(super) backoff_state: String,
    pub(super) health_level: String,
    pub(super) status: String,
    pub(super) items_seen: usize,
    pub(super) events_written: usize,
    pub(super) duplicate_events_skipped: usize,
    pub(super) error: Option<String>,
}

impl SourceHealthRecord {
    fn new(input: HealthRecordInput<'_>) -> Self {
        let status = input.status.as_str();
        let health_level = input.status.health_level();
        let health_event_id = format!(
            "src_health_{}",
            short_hash(&format!(
                "{}:{}:{}:{}",
                input.source.source_id, input.checked_at_ms, status, input.duplicate_events_skipped
            ))
        );
        Self {
            schema_version: "source_health_v1".to_owned(),
            health_event_id,
            source_id: input.source.source_id.clone(),
            source_category: input.source.source_category.clone(),
            fetch_method: input.source.fetch_method.clone(),
            adapter: input.source.adapter.clone(),
            checked_at_ms: input.checked_at_ms,
            observed_at_ms: input.observed_at_ms,
            fetch_status: status.to_owned(),
            http_status_or_error: input.error.clone(),
            latency_ms: input.latency_ms,
            items_fetched: input.items_seen,
            items_emitted: input.events_written,
            dedup_dropped: input.duplicate_events_skipped,
            backoff_state: input.backoff_state.as_str().to_owned(),
            health_level: health_level.to_owned(),
            status: status.to_owned(),
            items_seen: input.items_seen,
            events_written: input.events_written,
            duplicate_events_skipped: input.duplicate_events_skipped,
            error: input.error,
        }
    }
}

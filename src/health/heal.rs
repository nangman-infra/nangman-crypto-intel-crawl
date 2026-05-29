use super::hash::short_hash;
use crate::registry::Source;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceHealRecord {
    pub(super) schema_version: String,
    pub(super) heal_event_id: String,
    pub(super) source_id: String,
    pub(super) observed_at_ms: i64,
    pub(super) failure_type: String,
    pub(super) heal_action: String,
    pub(super) attempt_count: usize,
    pub(super) next_retry_at_ms: Option<i64>,
    pub(super) heal_status: String,
}

impl SourceHealRecord {
    pub(crate) fn retry_after_failure(
        source: &Source,
        observed_at_ms: i64,
        error: &str,
        retry_interval_ms: Option<u64>,
    ) -> Self {
        let failure_type = classify_failure(error);
        let next_retry_at_ms = retry_interval_ms.map(|interval| observed_at_ms + interval as i64);
        let heal_event_id = format!(
            "src_heal_{}",
            short_hash(&format!(
                "{}:{}:{}",
                source.source_id, observed_at_ms, failure_type
            ))
        );

        Self {
            schema_version: "source_heal_event_v1".to_owned(),
            heal_event_id,
            source_id: source.source_id.clone(),
            observed_at_ms,
            failure_type: failure_type.to_owned(),
            heal_action: "retry_with_backoff".to_owned(),
            attempt_count: 1,
            next_retry_at_ms,
            heal_status: "scheduled".to_owned(),
        }
    }
}

fn classify_failure(error: &str) -> &'static str {
    let lower = error.to_lowercase();
    if lower.contains("429") || lower.contains("rate") || lower.contains("too many requests") {
        "rate_limited"
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout"
    } else if lower.contains("did not return an xml feed")
        || lower.contains("parse")
        || lower.contains("expected value")
        || lower.contains("invalid json")
    {
        "parse_failed"
    } else if lower.contains("challenge") || lower.contains("auth") {
        "auth_not_supported"
    } else if lower.contains("no usable") || lower.contains("no_items") {
        "content_empty"
    } else {
        "source_unreachable"
    }
}

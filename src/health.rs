use crate::registry::Source;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceHealthRecord {
    schema_version: String,
    health_event_id: String,
    source_id: String,
    source_category: String,
    fetch_method: String,
    adapter: Option<String>,
    checked_at_ms: i64,
    observed_at_ms: i64,
    fetch_status: String,
    http_status_or_error: Option<String>,
    latency_ms: i64,
    items_fetched: usize,
    items_emitted: usize,
    dedup_dropped: usize,
    backoff_state: String,
    health_level: String,
    status: String,
    items_seen: usize,
    events_written: usize,
    duplicate_events_skipped: usize,
    error: Option<String>,
}

impl SourceHealthRecord {
    pub(crate) fn ok(
        source: &Source,
        checked_at_ms: i64,
        observed_at_ms: i64,
        items_seen: usize,
        events_written: usize,
        duplicate_events_skipped: usize,
    ) -> Self {
        let status = if items_seen == 0 { "no_items" } else { "ok" };
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status,
            items_seen,
            events_written,
            duplicate_events_skipped,
            latency_ms: observed_at_ms.saturating_sub(checked_at_ms),
            error: None,
            backoff_state: "none",
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
            status: "failed",
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: observed_at_ms.saturating_sub(checked_at_ms),
            error: Some(error),
            backoff_state: "scheduled",
        })
    }

    pub(crate) fn not_modified(source: &Source, checked_at_ms: i64, observed_at_ms: i64) -> Self {
        Self::new(HealthRecordInput {
            source,
            checked_at_ms,
            observed_at_ms,
            status: "not_modified",
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: observed_at_ms.saturating_sub(checked_at_ms),
            error: None,
            backoff_state: "none",
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
            status: "skipped_backoff",
            items_seen: 0,
            events_written: 0,
            duplicate_events_skipped: 0,
            latency_ms: 0,
            error: backoff_until_ms.map(|until| format!("backoff_until_ms={until}")),
            backoff_state: "active",
        })
    }

    fn new(input: HealthRecordInput<'_>) -> Self {
        let health_level = match input.status {
            "ok" | "not_modified" => "healthy",
            "no_items" | "skipped_backoff" => "degraded",
            _ => "blocked",
        };
        let health_event_id = format!(
            "src_health_{}",
            short_hash(&format!(
                "{}:{}:{}:{}",
                input.source.source_id,
                input.checked_at_ms,
                input.status,
                input.duplicate_events_skipped
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
            fetch_status: input.status.to_owned(),
            http_status_or_error: input.error.clone(),
            latency_ms: input.latency_ms,
            items_fetched: input.items_seen,
            items_emitted: input.events_written,
            dedup_dropped: input.duplicate_events_skipped,
            backoff_state: input.backoff_state.to_owned(),
            health_level: health_level.to_owned(),
            status: input.status.to_owned(),
            items_seen: input.items_seen,
            events_written: input.events_written,
            duplicate_events_skipped: input.duplicate_events_skipped,
            error: input.error,
        }
    }
}

struct HealthRecordInput<'a> {
    source: &'a Source,
    checked_at_ms: i64,
    observed_at_ms: i64,
    status: &'a str,
    items_seen: usize,
    events_written: usize,
    duplicate_events_skipped: usize,
    latency_ms: i64,
    error: Option<String>,
    backoff_state: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceHealRecord {
    schema_version: String,
    heal_event_id: String,
    source_id: String,
    observed_at_ms: i64,
    failure_type: String,
    heal_action: String,
    attempt_count: usize,
    next_retry_at_ms: Option<i64>,
    heal_status: String,
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

fn short_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .chars()
        .take(24)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{AppliesToAssets, Source};

    fn source() -> Source {
        Source {
            source_id: "news".to_owned(),
            source_category: "news".to_owned(),
            source_name: "News".to_owned(),
            source_url: "https://example.com/rss.xml".to_owned(),
            fetch_method: "rss".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T1".to_owned(),
            cadence_tier: "medium".to_owned(),
            language_hint: "en".to_owned(),
            enabled: true,
            source_state: None,
            activation_blocker: None,
            top50_relevance_mode: "symbol_alias_match".to_owned(),
            applies_to_assets: AppliesToAssets::All("all_major_50".to_owned()),
        }
    }

    #[test]
    fn health_record_contains_contract_fields() {
        let record = SourceHealthRecord::ok(&source(), 10, 30, 2, 1, 1);

        assert_eq!(record.fetch_status, "ok");
        assert_eq!(record.health_level, "healthy");
        assert_eq!(record.latency_ms, 20);
        assert_eq!(record.items_fetched, 2);
        assert_eq!(record.dedup_dropped, 1);
    }

    #[test]
    fn heal_record_classifies_rate_limit() {
        let record =
            SourceHealRecord::retry_after_failure(&source(), 100, "returned HTTP 429", Some(60));

        assert_eq!(record.failure_type, "rate_limited");
        assert_eq!(record.heal_action, "retry_with_backoff");
        assert_eq!(record.next_retry_at_ms, Some(160));
    }

    #[test]
    fn heal_record_does_not_classify_url_extension_as_parse_failure() {
        let record = SourceHealRecord::retry_after_failure(
            &source(),
            100,
            "error sending request for url (https://127.0.0.1:1/rss.xml)",
            Some(60),
        );

        assert_eq!(record.failure_type, "source_unreachable");
    }
}

use crate::object_store::ObjectStore;
use crate::publisher::EventPublisher;
use serde::Serialize;
use serde_json::Value;
use std::error::Error;

const PENDING_OUTBOX_PREFIX: &str =
    "publish-outbox/status=pending/schema=raw_intel_event_created_v2/";

#[derive(Debug, Serialize)]
pub(crate) struct ReplaySummary {
    schema_version: String,
    pending_objects_scanned: usize,
    pending_records_seen: usize,
    records_published: usize,
    records_failed: usize,
    failures: Vec<String>,
}

impl ReplaySummary {
    fn new() -> Self {
        Self {
            schema_version: "pending_outbox_replay_summary_v1".to_owned(),
            pending_objects_scanned: 0,
            pending_records_seen: 0,
            records_published: 0,
            records_failed: 0,
            failures: Vec::new(),
        }
    }
}

pub(crate) async fn replay_pending_outbox(
    object_store: &ObjectStore,
    publisher: &EventPublisher,
) -> Result<ReplaySummary, Box<dyn Error>> {
    let mut summary = ReplaySummary::new();
    let keys = object_store.list_keys(PENDING_OUTBOX_PREFIX).await?;
    for key in keys.into_iter().filter(|key| key.ends_with(".jsonl")) {
        summary.pending_objects_scanned += 1;
        let raw = String::from_utf8(object_store.get_bytes(&key).await?)?;
        for (event_id, payload) in collect_pending_payloads(&key, &raw, &mut summary) {
            match publisher.publish(&event_id, &payload).await {
                Ok(()) => summary.records_published += 1,
                Err(error) => {
                    summary.records_failed += 1;
                    summary
                        .failures
                        .push(format!("{key} publish failed for {event_id}: {error}"));
                }
            }
        }
    }
    publisher.flush().await?;
    Ok(summary)
}

fn collect_pending_payloads(
    key: &str,
    raw: &str,
    summary: &mut ReplaySummary,
) -> Vec<(String, Value)> {
    let mut payloads = Vec::new();
    for (line_index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        summary.pending_records_seen += 1;
        let payload = match serde_json::from_str::<Value>(line) {
            Ok(payload) => payload,
            Err(error) => {
                summary.records_failed += 1;
                summary
                    .failures
                    .push(format!("{key}:{} invalid JSON: {error}", line_index + 1));
                continue;
            }
        };
        let Some(event_id) = payload.get("event_id").and_then(Value::as_str) else {
            summary.records_failed += 1;
            summary
                .failures
                .push(format!("{key}:{} missing event_id", line_index + 1));
            continue;
        };
        payloads.push((event_id.to_owned(), payload));
    }
    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_pending_payloads_skips_invalid_records() {
        let mut summary = ReplaySummary::new();

        let payloads = collect_pending_payloads(
            "publish-outbox/status=pending/schema=raw_intel_event_created_v2/dt=2026-05-20/part-000001.jsonl",
            r#"
            {"event_id":"evt-1","source_id":"news"}
            {"source_id":"missing-event-id"}
            not-json
            {"event_id":"evt-2","source_id":"news"}
            "#,
            &mut summary,
        );

        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0].0, "evt-1");
        assert_eq!(payloads[1].0, "evt-2");
        assert_eq!(summary.pending_records_seen, 4);
        assert_eq!(summary.records_failed, 2);
        assert_eq!(summary.failures.len(), 2);
    }

    #[test]
    fn replay_summary_starts_empty() {
        let summary = ReplaySummary::new();

        assert_eq!(summary.schema_version, "pending_outbox_replay_summary_v1");
        assert_eq!(summary.pending_objects_scanned, 0);
        assert_eq!(summary.pending_records_seen, 0);
        assert_eq!(summary.records_published, 0);
        assert_eq!(summary.records_failed, 0);
        assert!(summary.failures.is_empty());
    }
}

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
            match publisher.publish(event_id, &payload).await {
                Ok(()) => summary.records_published += 1,
                Err(error) => {
                    summary.records_failed += 1;
                    summary
                        .failures
                        .push(format!("{key}:{} publish failed: {error}", line_index + 1));
                }
            }
        }
    }
    publisher.flush().await?;
    Ok(summary)
}

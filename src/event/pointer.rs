use super::RawIntelEvent;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RawIntelEventCreatedPointer {
    schema_version: String,
    event_id: String,
    source_id: String,
    source_category: String,
    fetched_at_ms: i64,
    published_at_ms: Option<i64>,
    created_at_ms: i64,
    content_hash: String,
    dedup_key: String,
    dedup_decision: String,
    duplicate_of_event_id: Option<String>,
    symbol_candidates: Vec<String>,
    top50_relevance: String,
    storage_ref: RawIntelEventStorageRef,
}

impl RawIntelEventCreatedPointer {
    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RawIntelEventStorageRef {
    kind: String,
    endpoint_alias: String,
    bucket: String,
    key: String,
    line_number: usize,
    byte_offset: usize,
    byte_length: usize,
    content_sha256: String,
}

pub(crate) fn build_raw_intel_event_created_pointer(
    event: &RawIntelEvent,
    storage_ref: RawIntelEventStorageRef,
    created_at_ms: i64,
) -> RawIntelEventCreatedPointer {
    RawIntelEventCreatedPointer {
        schema_version: "raw_intel_event_created_v2".to_owned(),
        event_id: event.event_id.clone(),
        source_id: event.source_id.clone(),
        source_category: event.source_category.clone(),
        fetched_at_ms: event.fetched_at_ms,
        published_at_ms: event.published_at_ms,
        created_at_ms,
        content_hash: event.content_hash.clone(),
        dedup_key: event.dedup_key.clone(),
        dedup_decision: event.dedup_decision.clone(),
        duplicate_of_event_id: event.duplicate_of_event_id.clone(),
        symbol_candidates: event.symbol_candidates.clone(),
        top50_relevance: event.top50_relevance.clone(),
        storage_ref,
    }
}

impl RawIntelEventStorageRef {
    pub(crate) fn aws_s3_jsonl_record(
        bucket: String,
        key: String,
        line_number: usize,
        byte_offset: usize,
        byte_length: usize,
        content_sha256: String,
    ) -> Self {
        Self {
            kind: "aws_s3_jsonl_record".to_owned(),
            endpoint_alias: "aws-s3-primary".to_owned(),
            bucket,
            key,
            line_number,
            byte_offset,
            byte_length,
            content_sha256,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::build_raw_intel_event;
    use crate::item::FeedItem;
    use crate::registry::{AppliesToAssets, Source};

    #[test]
    fn pointer_keeps_event_identity_and_storage_ref() {
        let source = Source {
            source_id: "project_btc".to_owned(),
            source_category: "project_notice".to_owned(),
            source_name: "Bitcoin".to_owned(),
            source_url: "https://example.com/feed.xml".to_owned(),
            fetch_method: "rss".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T0".to_owned(),
            cadence_tier: "low".to_owned(),
            language_hint: "en".to_owned(),
            enabled: true,
            source_state: None,
            activation_blocker: None,
            top50_relevance_mode: "direct_asset".to_owned(),
            applies_to_assets: AppliesToAssets::List(vec!["BTC".to_owned()]),
        };
        let item = FeedItem {
            id: Some("g1".to_owned()),
            title: "Bitcoin Core update".to_owned(),
            body: "Body".to_owned(),
            url: "https://example.com/b".to_owned(),
            author: None,
            published_at: None,
            historical_source_depth: None,
            backfill_window_start_ms: None,
            backfill_window_end_ms: None,
            source_time_range_verified: None,
        };
        let event = build_raw_intel_event(&source, &item, &[], 10);
        let storage_ref = RawIntelEventStorageRef::aws_s3_jsonl_record(
            "intel-crawl-app-l0".to_owned(),
            "raw-intel-events/schema=raw_intel_event_v1/dt=2026-05-07/hour=10/source_category=project_notice/source_id=project_btc/run_id=test/part-000001.jsonl".to_owned(),
            1,
            0,
            100,
            "sha256:abc".to_owned(),
        );
        let pointer = build_raw_intel_event_created_pointer(&event, storage_ref, 20);

        assert_eq!(pointer.event_id, event.event_id);
        assert_eq!(pointer.source_id, "project_btc");
        assert_eq!(pointer.schema_version, "raw_intel_event_created_v2");
        assert_eq!(pointer.storage_ref.kind, "aws_s3_jsonl_record");
    }
}

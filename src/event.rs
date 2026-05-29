use serde::{Deserialize, Serialize};

mod builder;
mod pointer;

pub(crate) use builder::build_raw_intel_event;
pub(crate) use pointer::{RawIntelEventStorageRef, build_raw_intel_event_created_pointer};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RawIntelEvent {
    event_id: String,
    source_id: String,
    source_category: String,
    source_name: String,
    fetched_at_ms: i64,
    published_at_ms: Option<i64>,
    observed_at_ms: i64,
    language: String,
    title: String,
    body: String,
    url: String,
    author_or_channel: Option<String>,
    trust_tier: String,
    cadence_tier: String,
    content_hash: String,
    dedup_key: String,
    exact_source_key: String,
    canonical_url: String,
    canonical_url_hash: String,
    normalized_content_hash: String,
    simhash64: String,
    dedup_decision: String,
    duplicate_of_event_id: Option<String>,
    symbol_candidates: Vec<String>,
    event_category_hint: Option<String>,
    top50_relevance: String,
    content_kind: String,
    content_quality: String,
    content_quality_score: u8,
    source_quality: String,
    source_relevance_scope: String,
    direct_asset_count: usize,
    matched_asset_count: usize,
    historical_source_depth: String,
    backfill_window_start_ms: Option<i64>,
    backfill_window_end_ms: Option<i64>,
    source_time_range_verified: bool,
    schema_version: String,
}

impl RawIntelEvent {
    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }

    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }

    pub(crate) fn source_category(&self) -> &str {
        &self.source_category
    }

    pub(crate) fn fetched_at_ms(&self) -> i64 {
        self.fetched_at_ms
    }

    pub(crate) fn content_hash(&self) -> &str {
        &self.content_hash
    }

    pub(crate) fn dedup_key(&self) -> &str {
        &self.dedup_key
    }

    pub(crate) fn exact_source_key(&self) -> &str {
        &self.exact_source_key
    }

    pub(crate) fn canonical_url_hash(&self) -> &str {
        &self.canonical_url_hash
    }

    pub(crate) fn normalized_content_hash(&self) -> &str {
        &self.normalized_content_hash
    }

    pub(crate) fn simhash64_value(&self) -> u64 {
        u64::from_str_radix(&self.simhash64, 16).unwrap_or(0)
    }

    pub(crate) fn dedup_decision(&self) -> &str {
        &self.dedup_decision
    }

    pub(crate) fn duplicate_of_event_id(&self) -> Option<&str> {
        self.duplicate_of_event_id.as_deref()
    }

    pub(crate) fn set_dedup_outcome(
        &mut self,
        dedup_decision: &str,
        duplicate_of_event_id: Option<String>,
    ) {
        self.dedup_decision = dedup_decision.to_owned();
        self.duplicate_of_event_id = duplicate_of_event_id;
    }
}

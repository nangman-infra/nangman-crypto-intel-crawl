use super::classification::{is_community_source, is_derivatives_snapshot_source};
use super::{SourceBalancePolicy, SourceRunStats};
use crate::registry::Source;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceBalanceRecord {
    schema_version: String,
    observed_at_ms: i64,
    source_id: String,
    source_category: String,
    fetch_method: String,
    adapter: Option<String>,
    balance_class: String,
    items_seen: usize,
    candidates_after_dedup: usize,
    events_emitted: usize,
    duplicates_skipped: usize,
    suppressed_by_balance: usize,
    suppression_reasons: BTreeMap<String, usize>,
    derivatives_max_events_per_run: usize,
    derivatives_max_events_per_source: usize,
    configured_derivatives_max_events_per_run: usize,
    configured_derivatives_max_events_per_source: usize,
    community_max_events_per_run: usize,
    community_max_events_per_source: usize,
}

impl SourceBalanceRecord {
    pub(crate) fn new(
        source: &Source,
        stats: SourceRunStats,
        policy: SourceBalancePolicy,
        observed_at_ms: i64,
    ) -> Self {
        Self {
            schema_version: "source_balance_v1".to_owned(),
            observed_at_ms,
            source_id: source.source_id.clone(),
            source_category: source.source_category.clone(),
            fetch_method: source.fetch_method.clone(),
            adapter: source.adapter.clone(),
            balance_class: if is_derivatives_snapshot_source(source) {
                "derivatives_snapshot"
            } else if is_community_source(source) {
                "community_reaction"
            } else {
                "standard_source"
            }
            .to_owned(),
            items_seen: stats.items_seen,
            candidates_after_dedup: stats.candidates_after_dedup,
            events_emitted: stats.events_emitted,
            duplicates_skipped: stats.duplicates_skipped,
            suppressed_by_balance: stats.suppressed_by_balance,
            suppression_reasons: stats.suppression_reasons,
            derivatives_max_events_per_run: policy.effective_derivatives_max_events_per_run(),
            derivatives_max_events_per_source: policy.effective_derivatives_max_events_per_source(),
            configured_derivatives_max_events_per_run: policy.derivatives_max_events_per_run,
            configured_derivatives_max_events_per_source: policy.derivatives_max_events_per_source,
            community_max_events_per_run: policy.community_max_events_per_run,
            community_max_events_per_source: policy.community_max_events_per_source,
        }
    }
}

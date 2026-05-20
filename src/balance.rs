use crate::registry::Source;
use serde::Serialize;
use std::collections::BTreeMap;

const DERIVATIVES_SAFETY_MAX_EVENTS_PER_RUN: usize = 12;
const DERIVATIVES_SAFETY_MAX_EVENTS_PER_SOURCE: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBalancePolicy {
    pub(crate) derivatives_max_events_per_run: usize,
    pub(crate) derivatives_max_events_per_source: usize,
    pub(crate) community_max_events_per_run: usize,
    pub(crate) community_max_events_per_source: usize,
}

impl SourceBalancePolicy {
    pub(crate) fn effective_derivatives_max_events_per_run(&self) -> usize {
        self.derivatives_max_events_per_run
            .min(DERIVATIVES_SAFETY_MAX_EVENTS_PER_RUN)
    }

    pub(crate) fn effective_derivatives_max_events_per_source(&self) -> usize {
        self.derivatives_max_events_per_source
            .min(DERIVATIVES_SAFETY_MAX_EVENTS_PER_SOURCE)
    }

    pub(crate) fn effective_item_limit(&self, source: &Source, requested_limit: usize) -> usize {
        if is_derivatives_snapshot_source(source) {
            requested_limit.min(self.effective_derivatives_max_events_per_source())
        } else if is_community_source(source) {
            requested_limit.min(self.community_max_events_per_source)
        } else {
            requested_limit
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct SourceBalanceTracker {
    derivatives_emitted: usize,
    community_emitted: usize,
    category_emitted: BTreeMap<String, usize>,
}

impl SourceBalanceTracker {
    pub(crate) fn admit(&mut self, source: &Source, policy: SourceBalancePolicy) -> Admission {
        if is_derivatives_snapshot_source(source) {
            if self.derivatives_emitted >= policy.effective_derivatives_max_events_per_run() {
                return Admission::Suppress {
                    reason: "derivatives_snapshot_run_cap".to_owned(),
                };
            }
            self.derivatives_emitted += 1;
        } else if is_community_source(source) {
            if self.community_emitted >= policy.community_max_events_per_run {
                return Admission::Suppress {
                    reason: "community_reaction_run_cap".to_owned(),
                };
            }
            self.community_emitted += 1;
        }
        *self
            .category_emitted
            .entry(source.source_category.clone())
            .or_insert(0) += 1;
        Admission::Admit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Admission {
    Admit,
    Suppress { reason: String },
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceRunStats {
    pub(crate) items_seen: usize,
    pub(crate) candidates_after_dedup: usize,
    pub(crate) events_emitted: usize,
    pub(crate) duplicates_skipped: usize,
    pub(crate) suppressed_by_balance: usize,
    pub(crate) suppression_reasons: BTreeMap<String, usize>,
}

impl SourceRunStats {
    pub(crate) fn record_suppression(&mut self, reason: String) {
        self.suppressed_by_balance += 1;
        *self.suppression_reasons.entry(reason).or_insert(0) += 1;
    }
}

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

fn is_derivatives_snapshot_source(source: &Source) -> bool {
    source.source_category == "funding"
        && source.fetch_method == "rest_api"
        && !source.is_manual_backfill_source()
}

fn is_community_source(source: &Source) -> bool {
    source.source_category == "social"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::AppliesToAssets;

    #[test]
    fn caps_derivatives_globally_and_per_source() {
        let policy = SourceBalancePolicy {
            derivatives_max_events_per_run: 2,
            derivatives_max_events_per_source: 5,
            community_max_events_per_run: 30,
            community_max_events_per_source: 5,
        };
        let source = funding_source();
        let mut tracker = SourceBalanceTracker::default();

        assert_eq!(tracker.admit(&source, policy), Admission::Admit);
        assert_eq!(tracker.admit(&source, policy), Admission::Admit);
        assert_eq!(
            tracker.admit(&source, policy),
            Admission::Suppress {
                reason: "derivatives_snapshot_run_cap".to_owned()
            }
        );
        assert_eq!(policy.effective_item_limit(&source, 50), 5);
    }

    #[test]
    fn caps_community_globally_and_per_source() {
        let policy = SourceBalancePolicy {
            derivatives_max_events_per_run: 40,
            derivatives_max_events_per_source: 20,
            community_max_events_per_run: 1,
            community_max_events_per_source: 3,
        };
        let source = social_source();
        let mut tracker = SourceBalanceTracker::default();

        assert_eq!(tracker.admit(&source, policy), Admission::Admit);
        assert_eq!(
            tracker.admit(&source, policy),
            Admission::Suppress {
                reason: "community_reaction_run_cap".to_owned()
            }
        );
        assert_eq!(policy.effective_item_limit(&source, 50), 3);
    }

    #[test]
    fn clamps_derivatives_to_enterprise_safety_caps() {
        let policy = SourceBalancePolicy {
            derivatives_max_events_per_run: 40,
            derivatives_max_events_per_source: 20,
            community_max_events_per_run: 30,
            community_max_events_per_source: 5,
        };
        let source = funding_source();
        let mut tracker = SourceBalanceTracker::default();

        assert_eq!(policy.effective_derivatives_max_events_per_run(), 12);
        assert_eq!(policy.effective_derivatives_max_events_per_source(), 6);
        assert_eq!(policy.effective_item_limit(&source, 50), 6);
        for _ in 0..12 {
            assert_eq!(tracker.admit(&source, policy), Admission::Admit);
        }
        assert_eq!(
            tracker.admit(&source, policy),
            Admission::Suppress {
                reason: "derivatives_snapshot_run_cap".to_owned()
            }
        );
    }

    #[test]
    fn manual_funding_history_backfill_is_not_live_derivatives_capped() {
        let policy = SourceBalancePolicy {
            derivatives_max_events_per_run: 2,
            derivatives_max_events_per_source: 2,
            community_max_events_per_run: 30,
            community_max_events_per_source: 5,
        };
        let mut source = funding_source();
        source.source_id = "derivatives_binance_usdm_funding_rate_history_rest".to_owned();
        source.adapter = Some("binance_usdm_funding_rate_history".to_owned());
        source.enabled = false;
        source.source_state = Some("available_disabled".to_owned());

        let mut tracker = SourceBalanceTracker::default();

        assert_eq!(policy.effective_item_limit(&source, 1000), 1000);
        for _ in 0..20 {
            assert_eq!(tracker.admit(&source, policy), Admission::Admit);
        }
    }

    fn funding_source() -> Source {
        source("funding", "rest_api", Some("binance_usdm_open_interest"))
    }

    fn social_source() -> Source {
        source("social", "rss", None)
    }

    fn source(source_category: &str, fetch_method: &str, adapter: Option<&str>) -> Source {
        Source {
            source_id: source_category.to_owned(),
            source_category: source_category.to_owned(),
            source_name: source_category.to_owned(),
            source_url: "https://example.com".to_owned(),
            fetch_method: fetch_method.to_owned(),
            adapter: adapter.map(str::to_owned),
            max_items_per_run: Some(50),
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
}

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

fn funding_source() -> crate::registry::Source {
    source("funding", "rest_api", Some("binance_usdm_open_interest"))
}

fn social_source() -> crate::registry::Source {
    source("social", "rss", None)
}

fn source(
    source_category: &str,
    fetch_method: &str,
    adapter: Option<&str>,
) -> crate::registry::Source {
    crate::registry::Source {
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

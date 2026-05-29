use super::*;
use crate::registry::{AppliesToAssets, Source};

#[test]
fn direct_asset_source_marks_event_relevant() {
    let source = Source {
        source_id: "project_eth".to_owned(),
        source_category: "project_notice".to_owned(),
        source_name: "Ethereum".to_owned(),
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
        applies_to_assets: AppliesToAssets::List(vec!["ETH".to_owned()]),
    };
    let item = FeedItem {
        id: None,
        title: "Protocol update".to_owned(),
        body: "<p>Upgrade</p>".to_owned(),
        url: "https://example.com/a".to_owned(),
        author: None,
        published_at: None,
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    };

    let event = build_raw_intel_event(&source, &item, &[], 1);

    assert_eq!(event.symbol_candidates, vec!["ETH"]);
    assert_eq!(event.top50_relevance, "relevant");
    assert_eq!(event.body, "Upgrade");
    assert_eq!(event.event_category_hint.as_deref(), Some("project_notice"));
    assert_eq!(event.content_kind, "project_notice");
    assert_eq!(event.source_quality, "trusted_direct");
    assert_eq!(event.source_relevance_scope, "direct_asset");
    assert_eq!(event.direct_asset_count, 1);
    assert_eq!(event.historical_source_depth, "feed_retained");
    assert!(!event.source_time_range_verified);
}

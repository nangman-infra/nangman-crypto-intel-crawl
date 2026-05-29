use super::*;

#[test]
fn detects_duplicate_keys() {
    let mut store = DedupStore::default();
    let event = test_event("https://example.com/a", "A title", "A body");

    assert_eq!(store.decide_and_insert(&event), DedupDecision::New);
    assert!(store.decide_and_insert(&event).is_skipped_duplicate());
}

#[test]
fn loads_dedup_keys_from_jsonl() {
    let store = DedupStore::from_jsonl(r#"{"schema_version":"dedup_index_v1","dedup_key":"a"}"#);

    assert_eq!(store.len(), 1);
}

#[test]
fn detects_cross_source_content_duplicates() {
    let mut store = DedupStore::default();
    let first = test_event(
        "https://example.com/a",
        "Binance lists TEST",
        "Trading starts",
    );
    let mut second = test_event(
        "https://other.example/news",
        "Binance lists TEST",
        "Trading starts",
    );

    assert_eq!(store.decide_and_insert(&first), DedupDecision::New);
    let decision = store.decide_and_insert(&second);
    second.set_dedup_outcome(decision.label(), decision.duplicate_of_event_id());

    assert!(decision.is_skipped_duplicate());
}

#[test]
fn detects_updates_for_same_source_item_with_changed_content() {
    let mut store = DedupStore::default();
    let first = test_event(
        "https://example.com/a",
        "Exchange maintenance",
        "Wallet maintenance starts at 10:00 UTC",
    );
    let second = test_event(
        "https://example.com/a",
        "Exchange maintenance",
        "Wallet maintenance starts at 10:30 UTC",
    );

    assert_eq!(store.decide_and_insert(&first), DedupDecision::New);

    let decision = store.decide_and_insert(&second);

    assert_eq!(
        decision,
        DedupDecision::UpdateOfExisting {
            duplicate_of_event_id: Some(first.event_id().to_owned())
        }
    );
    assert!(!decision.is_skipped_duplicate());
}

#[test]
fn loads_v2_hashes_and_detects_near_duplicates() {
    let known = test_event(
        "https://example.com/a",
        "Solana validator client releases urgent patch",
        "Operators should upgrade before the next epoch boundary.",
    );
    let raw = format!(
        r#"{{"event_id":"known-event","simhash64":"{:016x}"}}"#,
        known.simhash64_value()
    );
    let mut store = DedupStore::from_jsonl(&raw);
    let candidate = test_event(
        "https://other.example/b",
        "Solana validator client releases urgent patch",
        "Operators should upgrade before the next epoch boundary.",
    );

    let decision = store.decide_and_insert(&candidate);

    assert_eq!(
        decision,
        DedupDecision::NearDuplicate {
            duplicate_of_event_id: Some("known-event".to_owned())
        }
    );
}

#[test]
fn ignores_invalid_loaded_records() {
    let store = DedupStore::from_jsonl(
        r#"
            not json
            {"event_id":"missing-hashes"}
            {"event_id":"bad-simhash","simhash64":"not-hex"}
            "#,
    );

    assert_eq!(store.len(), 0);
}

fn test_event(url: &str, title: &str, body: &str) -> crate::event::RawIntelEvent {
    use crate::event::build_raw_intel_event;
    use crate::item::FeedItem;
    use crate::registry::{AppliesToAssets, Source};

    let source = Source {
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
    };
    let item = FeedItem {
        id: None,
        title: title.to_owned(),
        body: body.to_owned(),
        url: url.to_owned(),
        author: None,
        published_at: None,
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    };
    build_raw_intel_event(&source, &item, &[], 1)
}

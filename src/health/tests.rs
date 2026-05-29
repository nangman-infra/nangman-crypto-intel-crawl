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
fn cadence_skip_is_recorded_as_healthy_deferred_fetch() {
    let record = SourceHealthRecord::skipped_cadence(&source(), 10, 20, 1_000);

    assert_eq!(record.fetch_status, "skipped_cadence");
    assert_eq!(record.health_level, "healthy");
    assert_eq!(
        record.http_status_or_error.as_deref(),
        Some("next_due_at_ms=1000")
    );
    assert_eq!(record.latency_ms, 0);
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

use super::keys::state_object_key;
use super::state::SourceFetchState;
use crate::fetch::FetchMetadata;
use crate::registry::{AppliesToAssets, Source};

#[test]
fn backs_off_exponentially_after_failures() {
    let source = source();
    let mut state = SourceFetchState::new(&source);

    state.record_failure("HTTP 502", 1_000);
    assert_eq!(state.backoff_until_ms(), Some(61_000));

    state.record_failure("HTTP 502", 2_000);
    assert_eq!(state.backoff_until_ms(), Some(122_000));
}

#[test]
fn records_conditional_get_metadata() {
    let source = source();
    let mut state = SourceFetchState::new(&source);
    let metadata = FetchMetadata {
        http_status: 200,
        etag: Some("\"abc\"".to_owned()),
        last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned()),
    };

    state.record_success(&metadata, 1_000, 1);

    let headers = state.cache_headers();
    assert_eq!(headers.etag.as_deref(), Some("\"abc\""));
    assert_eq!(
        headers.last_modified.as_deref(),
        Some("Wed, 21 Oct 2015 07:28:00 GMT")
    );
}

#[test]
fn records_not_modified_without_losing_cache_metadata() {
    let source = source();
    let mut state = SourceFetchState::new(&source);
    state.record_success(
        &FetchMetadata {
            http_status: 200,
            etag: Some("\"abc\"".to_owned()),
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned()),
        },
        1_000,
        1,
    );

    state.record_not_modified(
        &FetchMetadata {
            http_status: 304,
            etag: None,
            last_modified: None,
        },
        2_000,
    );

    assert_eq!(state.last_http_status, Some(304));
    assert_eq!(state.last_checked_at_ms, Some(2_000));
    assert_eq!(state.last_success_at_ms(), Some(2_000));
    assert_eq!(state.unchanged_count, 1);
    assert_eq!(state.failure_count, 0);
    assert_eq!(state.backoff_until_ms(), None);
    let headers = state.cache_headers();
    assert_eq!(headers.etag.as_deref(), Some("\"abc\""));
    assert_eq!(
        headers.last_modified.as_deref(),
        Some("Wed, 21 Oct 2015 07:28:00 GMT")
    );
}

#[test]
fn clears_backoff_after_success() {
    let source = source();
    let mut state = SourceFetchState::new(&source);
    state.record_failure("HTTP 503", 1_000);
    assert!(state.is_backing_off(1_500));

    state.record_success(
        &FetchMetadata {
            http_status: 200,
            etag: None,
            last_modified: None,
        },
        2_000,
        0,
    );

    assert_eq!(state.failure_count, 0);
    assert_eq!(state.backoff_until_ms(), None);
    assert!(!state.is_backing_off(2_001));
}

#[test]
fn state_key_sanitizes_source_id() {
    assert_eq!(
        state_object_key("news/binance announcements"),
        "source-fetch-state/schema=source_fetch_state_v1/source_id=news_binance_announcements/state.json"
    );
}

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

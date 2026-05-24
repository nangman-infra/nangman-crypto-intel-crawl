use crate::item::FeedItem;
use crate::normalization::{content_fingerprint, hash_hex, normalize_text_for_dedup};
use crate::registry::Source;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

pub(crate) fn build_raw_intel_event(
    source: &Source,
    item: &FeedItem,
    matched_assets: &[String],
    fetched_at_ms: i64,
) -> RawIntelEvent {
    let published_at_ms = item.published_at.as_deref().and_then(parse_published_at_ms);
    let body = clean_text(&item.body);
    let title = clean_text(&item.title);
    let fingerprint = content_fingerprint(&title, &body, &item.url);
    let dedupe_basis = item
        .id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&fingerprint.canonical_url);
    let normalized_dedupe_basis = normalize_text_for_dedup(dedupe_basis);
    let exact_source_key = hash_hex(&format!("{}:{normalized_dedupe_basis}", source.source_id));
    let content_hash = hash_hex(&format!(
        "{}\n{}",
        fingerprint.normalized_text_hash, fingerprint.canonical_url_hash
    ));
    let dedup_key = format!(
        "{}:{exact_source_key}:{}",
        source.source_id, fingerprint.normalized_text_hash
    );
    let mut candidates = BTreeSet::new();
    for asset in source.direct_assets() {
        candidates.insert(asset.to_owned());
    }
    for asset in matched_assets {
        candidates.insert(asset.to_owned());
    }
    let symbol_candidates = candidates.into_iter().collect::<Vec<_>>();
    let top50_relevance = if symbol_candidates.is_empty() {
        "unknown"
    } else {
        "relevant"
    }
    .to_owned();
    let direct_asset_count = source.direct_assets().len();
    let matched_asset_count = matched_assets.len();
    let content_kind = content_kind(source).to_owned();
    let content_quality = content_quality(source, &body).to_owned();
    let content_quality_score = content_quality_score(
        source,
        &content_quality,
        direct_asset_count,
        matched_asset_count,
    );
    let source_quality = source_quality(source, direct_asset_count, matched_asset_count).to_owned();
    let source_relevance_scope =
        source_relevance_scope(source, direct_asset_count, matched_asset_count).to_owned();

    RawIntelEvent {
        event_id: format!("intel_evt_{}", short_hash(&dedup_key)),
        source_id: source.source_id.clone(),
        source_category: source.source_category.clone(),
        source_name: source.source_name.clone(),
        fetched_at_ms,
        published_at_ms,
        observed_at_ms: published_at_ms.unwrap_or(fetched_at_ms),
        language: source.language_hint.clone(),
        title,
        body,
        url: item.url.clone(),
        author_or_channel: item.author.clone(),
        trust_tier: source.trust_tier.clone(),
        cadence_tier: source.cadence_tier.clone(),
        content_hash,
        dedup_key,
        exact_source_key,
        canonical_url: fingerprint.canonical_url,
        canonical_url_hash: fingerprint.canonical_url_hash,
        normalized_content_hash: fingerprint.normalized_text_hash,
        simhash64: format!("{:016x}", fingerprint.simhash64),
        dedup_decision: "new".to_owned(),
        duplicate_of_event_id: None,
        symbol_candidates,
        event_category_hint: Some(event_category_hint(source).to_owned()),
        top50_relevance,
        content_kind,
        content_quality,
        content_quality_score,
        source_quality,
        source_relevance_scope,
        direct_asset_count,
        matched_asset_count,
        historical_source_depth: item
            .historical_source_depth
            .clone()
            .unwrap_or_else(|| historical_source_depth(source).to_owned()),
        backfill_window_start_ms: item.backfill_window_start_ms,
        backfill_window_end_ms: item.backfill_window_end_ms,
        source_time_range_verified: item.source_time_range_verified.unwrap_or(false),
        schema_version: "raw_intel_event_v1".to_owned(),
    }
}

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
    pub(crate) fn legacy_raw_jsonl_record(
        bucket: String,
        key: String,
        line_number: usize,
        byte_offset: usize,
        byte_length: usize,
        content_sha256: String,
    ) -> Self {
        Self {
            kind: "rustfs_jsonl_record".to_owned(),
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

fn parse_published_at_ms(value: &str) -> Option<i64> {
    if let Ok(timestamp_ms) = value.parse::<i64>() {
        return Some(timestamp_ms);
    }

    DateTime::parse_from_rfc2822(value)
        .or_else(|_| DateTime::parse_from_rfc3339(value))
        .map(|date| date.with_timezone(&Utc).timestamp_millis())
        .ok()
}

fn clean_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short_hash(value: &str) -> String {
    hash_hex(value).chars().take(24).collect()
}

fn event_category_hint(source: &Source) -> &'static str {
    match source.adapter.as_deref() {
        Some("binance_cms_announcement_list")
            if source.source_id.contains("listing") && !source.source_id.contains("delisting") =>
        {
            "exchange_listing"
        }
        Some("binance_cms_announcement_list") if source.source_id.contains("delisting") => {
            "exchange_delisting"
        }
        Some("binance_cms_announcement_list") if source.source_id.contains("maintenance") => {
            "exchange_maintenance"
        }
        Some("binance_usdm_funding_rate_latest") => "funding_rate_snapshot",
        Some("binance_usdm_funding_rate_history") => "funding_rate_history",
        Some("binance_usdm_open_interest") => "open_interest_snapshot",
        Some("binance_usdm_open_interest_hist_recent") => "open_interest_snapshot_recent",
        _ => match source.source_category.as_str() {
            "governance" => "governance_update",
            "developer_activity" => "developer_release",
            "project_notice" => "project_notice",
            "social" => "community_reaction",
            "news" => "news_article",
            "exchange_notice" => "exchange_notice",
            "funding" => "derivatives_snapshot",
            _ => "raw_intel_observation",
        },
    }
}

fn historical_source_depth(source: &Source) -> &'static str {
    match source.adapter.as_deref() {
        Some("binance_usdm_funding_rate_history") => "range_queryable",
        Some("binance_usdm_open_interest_hist_recent") => "recent_1m",
        Some("binance_usdm_funding_rate_latest") | Some("binance_usdm_open_interest") => {
            "live_only"
        }
        Some("binance_cms_announcement_list") => "feed_retained",
        _ if source.fetch_method == "rss" => "feed_retained",
        _ => "unknown",
    }
}

fn content_kind(source: &Source) -> &'static str {
    match source.source_category.as_str() {
        "news" => "news_article",
        "social" => "community_reaction",
        "funding" => "derivatives_snapshot",
        "exchange_notice" => "exchange_notice",
        "project_notice" => "project_notice",
        "governance" => "governance_update",
        "developer_activity" => "developer_activity",
        _ => "raw_intel_observation",
    }
}

fn content_quality(source: &Source, body: &str) -> &'static str {
    if source.source_category == "funding" {
        return "numeric_observation";
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        "title_only"
    } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
        "metadata_fallback"
    } else if trimmed.chars().count() < 120 {
        "short_text"
    } else {
        "full_text"
    }
}

fn content_quality_score(
    source: &Source,
    content_quality: &str,
    direct_asset_count: usize,
    matched_asset_count: usize,
) -> u8 {
    let mut score = match content_quality {
        "full_text" => 70,
        "short_text" => 55,
        "numeric_observation" => 50,
        "metadata_fallback" => 40,
        "title_only" => 25,
        _ => 35,
    };
    if source.trust_tier == "T0" {
        score += 15;
    } else if source.trust_tier == "T1" {
        score += 8;
    }
    if direct_asset_count > 0 {
        score += 10;
    } else if matched_asset_count > 0 {
        score += 5;
    }
    score.min(100)
}

fn source_quality(
    source: &Source,
    direct_asset_count: usize,
    matched_asset_count: usize,
) -> &'static str {
    if source.source_category == "funding" {
        "market_snapshot"
    } else if source.source_category == "social" {
        "community_reaction"
    } else if source.trust_tier == "T0" && direct_asset_count > 0 {
        "trusted_direct"
    } else if source.trust_tier == "T0" {
        "trusted_official"
    } else if source.trust_tier == "T1" && matched_asset_count > 0 {
        "trusted_symbol_match"
    } else {
        "general_context"
    }
}

fn source_relevance_scope(
    source: &Source,
    direct_asset_count: usize,
    matched_asset_count: usize,
) -> &'static str {
    if direct_asset_count > 0 {
        "direct_asset"
    } else if matched_asset_count > 0 {
        "symbol_alias_match"
    } else if source.top50_relevance_mode == "symbol_alias_match" {
        "global_symbol_scan"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
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
        let storage_ref = RawIntelEventStorageRef::legacy_raw_jsonl_record(
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
        assert_eq!(pointer.storage_ref.kind, "rustfs_jsonl_record");
    }
}

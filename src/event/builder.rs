mod classification;
mod quality;
mod text;

use super::RawIntelEvent;
use crate::event::builder::classification::{
    content_kind, event_category_hint, historical_source_depth, source_quality,
    source_relevance_scope,
};
use crate::event::builder::quality::{content_quality, content_quality_score};
use crate::event::builder::text::{clean_text, parse_published_at_ms, short_hash};
use crate::item::FeedItem;
use crate::normalization::{content_fingerprint, hash_hex, normalize_text_for_dedup};
use crate::registry::Source;
use std::collections::BTreeSet;

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

#[cfg(test)]
mod tests;

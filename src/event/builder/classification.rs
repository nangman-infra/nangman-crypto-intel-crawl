use crate::registry::Source;

pub(super) fn event_category_hint(source: &Source) -> &'static str {
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

pub(super) fn historical_source_depth(source: &Source) -> &'static str {
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

pub(super) fn content_kind(source: &Source) -> &'static str {
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

pub(super) fn source_quality(
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

pub(super) fn source_relevance_scope(
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

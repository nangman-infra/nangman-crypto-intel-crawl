use crate::registry::{AppliesToAssets, Source};
use std::collections::BTreeMap;

pub(in crate::coverage) fn source_has_direct_asset(source: &Source, asset: &str) -> bool {
    source
        .direct_assets()
        .iter()
        .any(|source_asset| source_asset == asset)
}

pub(in crate::coverage) fn source_applies_to_asset(source: &Source, asset: &str) -> bool {
    match &source.applies_to_assets {
        AppliesToAssets::All(value) => value == "all_major_50",
        AppliesToAssets::List(values) => values.iter().any(|value| value == asset),
    }
}

pub(in crate::coverage) fn coverage_status(
    enabled_direct_source_count: usize,
    available_disabled_source_count: usize,
    enabled_global_source_count: usize,
) -> &'static str {
    if enabled_direct_source_count > 0 {
        "asset_specific_enabled"
    } else if available_disabled_source_count > 0 {
        "asset_specific_available_disabled"
    } else if enabled_global_source_count > 0 {
        "global_symbol_match_only"
    } else {
        "missing_enabled_source"
    }
}

pub(in crate::coverage) fn quality_gaps(
    enabled_category_counts: &BTreeMap<String, usize>,
    enabled_direct_source_count: usize,
    available_disabled_source_count: usize,
) -> Vec<String> {
    let mut gaps = Vec::new();
    if enabled_direct_source_count == 0 {
        gaps.push("missing_enabled_asset_specific_source".to_owned());
    }
    if available_disabled_source_count == 0
        && enabled_category_counts
            .get("social")
            .copied()
            .unwrap_or_default()
            == 0
    {
        gaps.push("missing_community_reaction_inventory".to_owned());
    }
    if !enabled_category_counts.contains_key("news") {
        gaps.push("missing_global_news_source".to_owned());
    }
    gaps
}

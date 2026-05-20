use crate::registry::{AppliesToAssets, Source, SourceRegistry};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SourceCoverageRecord {
    schema_version: String,
    observed_at_ms: i64,
    asset: String,
    reference_symbol_native: String,
    coverage_status: String,
    enabled_source_count: usize,
    enabled_direct_source_count: usize,
    enabled_global_source_count: usize,
    available_disabled_direct_source_count: usize,
    available_disabled_global_source_count: usize,
    enabled_category_counts: BTreeMap<String, usize>,
    direct_source_ids: Vec<String>,
    available_disabled_source_ids: Vec<String>,
    quality_gaps: Vec<String>,
}

pub(crate) fn build_source_coverage_report(
    registry: &SourceRegistry,
    observed_at_ms: i64,
) -> Vec<SourceCoverageRecord> {
    registry
        .universe_assets
        .iter()
        .map(|asset| {
            let mut enabled_category_counts = BTreeMap::new();
            let mut direct_source_ids = BTreeSet::new();
            let mut available_disabled_source_ids = BTreeSet::new();
            let mut enabled_direct_source_count = 0usize;
            let mut enabled_global_source_count = 0usize;
            let mut available_disabled_direct_source_count = 0usize;
            let mut available_disabled_global_source_count = 0usize;

            for source in &registry.sources {
                if !source_applies_to_asset(source, &asset.asset) {
                    continue;
                }
                if source.enabled {
                    *enabled_category_counts
                        .entry(source.source_category.clone())
                        .or_insert(0) += 1;
                    if source.direct_assets().contains(&asset.asset) {
                        enabled_direct_source_count += 1;
                        direct_source_ids.insert(source.source_id.clone());
                    } else {
                        enabled_global_source_count += 1;
                    }
                } else if source.source_state.as_deref() == Some("available_disabled") {
                    if source.direct_assets().contains(&asset.asset) {
                        available_disabled_direct_source_count += 1;
                    } else {
                        available_disabled_global_source_count += 1;
                    }
                    available_disabled_source_ids.insert(source.source_id.clone());
                }
            }

            let enabled_source_count = enabled_direct_source_count + enabled_global_source_count;
            let quality_gaps = quality_gaps(
                &enabled_category_counts,
                enabled_direct_source_count,
                available_disabled_direct_source_count + available_disabled_global_source_count,
            );
            SourceCoverageRecord {
                schema_version: "source_coverage_v1".to_owned(),
                observed_at_ms,
                asset: asset.asset.clone(),
                reference_symbol_native: asset.reference_symbol_native.clone(),
                coverage_status: coverage_status(
                    enabled_direct_source_count,
                    available_disabled_direct_source_count,
                    enabled_global_source_count,
                )
                .to_owned(),
                enabled_source_count,
                enabled_direct_source_count,
                enabled_global_source_count,
                available_disabled_direct_source_count,
                available_disabled_global_source_count,
                enabled_category_counts,
                direct_source_ids: direct_source_ids.into_iter().collect(),
                available_disabled_source_ids: available_disabled_source_ids.into_iter().collect(),
                quality_gaps,
            }
        })
        .collect()
}

fn source_applies_to_asset(source: &Source, asset: &str) -> bool {
    match &source.applies_to_assets {
        AppliesToAssets::All(value) => value == "all_major_50",
        AppliesToAssets::List(values) => values.iter().any(|value| value == asset),
    }
}

fn coverage_status(
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

fn quality_gaps(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{AppliesToAssets, Source, UniverseAsset};

    #[test]
    fn reports_asset_specific_and_global_coverage() {
        let registry = SourceRegistry {
            universe_assets: vec![
                UniverseAsset {
                    asset: "BTC".to_owned(),
                    reference_symbol_native: "BTCUSDT".to_owned(),
                    rss_seed_status: Some("asset_specific_verified".to_owned()),
                },
                UniverseAsset {
                    asset: "DOGE".to_owned(),
                    reference_symbol_native: "DOGEUSDT".to_owned(),
                    rss_seed_status: Some("global_news_only".to_owned()),
                },
            ],
            sources: vec![
                global_news(),
                direct_project("BTC", true),
                direct_project("DOGE", false),
            ],
        };

        let report = build_source_coverage_report(&registry, 10);

        assert_eq!(report[0].coverage_status, "asset_specific_enabled");
        assert_eq!(
            report[1].coverage_status,
            "asset_specific_available_disabled"
        );
        assert!(
            report[1]
                .quality_gaps
                .contains(&"missing_enabled_asset_specific_source".to_owned())
        );
    }

    fn global_news() -> Source {
        source(
            "news",
            "news",
            true,
            AppliesToAssets::All("all_major_50".to_owned()),
        )
    }

    fn direct_project(asset: &str, enabled: bool) -> Source {
        source(
            &format!("project_{asset}"),
            "project_notice",
            enabled,
            AppliesToAssets::List(vec![asset.to_owned()]),
        )
    }

    fn source(
        source_id: &str,
        source_category: &str,
        enabled: bool,
        applies_to_assets: AppliesToAssets,
    ) -> Source {
        Source {
            source_id: source_id.to_owned(),
            source_category: source_category.to_owned(),
            source_name: source_id.to_owned(),
            source_url: "https://example.com/feed.xml".to_owned(),
            fetch_method: "rss".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T1".to_owned(),
            cadence_tier: "medium".to_owned(),
            language_hint: "en".to_owned(),
            enabled,
            source_state: (!enabled).then(|| "available_disabled".to_owned()),
            activation_blocker: (!enabled).then(|| "not_active".to_owned()),
            top50_relevance_mode: "symbol_alias_match".to_owned(),
            applies_to_assets,
        }
    }
}

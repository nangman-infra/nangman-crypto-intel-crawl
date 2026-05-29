use crate::coverage::rules::{coverage_status, source_applies_to_asset};
use crate::coverage::state::AssetCoverage;
use crate::registry::{Source, SourceRegistry};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SourceCoverageRecord {
    pub(in crate::coverage) schema_version: String,
    pub(in crate::coverage) observed_at_ms: i64,
    pub(in crate::coverage) asset: String,
    pub(in crate::coverage) reference_symbol_native: String,
    pub(in crate::coverage) coverage_status: String,
    pub(in crate::coverage) enabled_source_count: usize,
    pub(in crate::coverage) enabled_direct_source_count: usize,
    pub(in crate::coverage) enabled_global_source_count: usize,
    pub(in crate::coverage) available_disabled_direct_source_count: usize,
    pub(in crate::coverage) available_disabled_global_source_count: usize,
    pub(in crate::coverage) enabled_category_counts: std::collections::BTreeMap<String, usize>,
    pub(in crate::coverage) direct_source_ids: Vec<String>,
    pub(in crate::coverage) available_disabled_source_ids: Vec<String>,
    pub(in crate::coverage) available_disabled_direct_sources: Vec<AvailableDisabledSourceDetail>,
    pub(in crate::coverage) quality_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(in crate::coverage) struct AvailableDisabledSourceDetail {
    pub(in crate::coverage) source_id: String,
    pub(in crate::coverage) source_name: String,
    pub(in crate::coverage) source_url: String,
    pub(in crate::coverage) source_category: String,
    pub(in crate::coverage) fetch_method: String,
    pub(in crate::coverage) trust_tier: String,
    pub(in crate::coverage) activation_blocker: Option<String>,
}

impl AvailableDisabledSourceDetail {
    pub(in crate::coverage) fn from_source(source: &Source) -> Self {
        Self {
            source_id: source.source_id.clone(),
            source_name: source.source_name.clone(),
            source_url: source.source_url.clone(),
            source_category: source.source_category.clone(),
            fetch_method: source.fetch_method.clone(),
            trust_tier: source.trust_tier.clone(),
            activation_blocker: source.activation_blocker.clone(),
        }
    }
}

pub(crate) fn build_source_coverage_report(
    registry: &SourceRegistry,
    observed_at_ms: i64,
) -> Vec<SourceCoverageRecord> {
    registry
        .universe_assets
        .iter()
        .map(|asset| build_asset_coverage_record(registry, observed_at_ms, asset))
        .collect()
}

fn build_asset_coverage_record(
    registry: &SourceRegistry,
    observed_at_ms: i64,
    asset: &crate::registry::UniverseAsset,
) -> SourceCoverageRecord {
    let coverage = collect_asset_coverage(registry, &asset.asset);
    let quality_gaps = coverage.quality_gaps();
    SourceCoverageRecord {
        schema_version: "source_coverage_v1".to_owned(),
        observed_at_ms,
        asset: asset.asset.clone(),
        reference_symbol_native: asset.reference_symbol_native.clone(),
        coverage_status: coverage_status(
            coverage.enabled_direct_source_count,
            coverage.available_disabled_direct_source_count,
            coverage.enabled_global_source_count,
        )
        .to_owned(),
        enabled_source_count: coverage.enabled_source_count(),
        enabled_direct_source_count: coverage.enabled_direct_source_count,
        enabled_global_source_count: coverage.enabled_global_source_count,
        available_disabled_direct_source_count: coverage.available_disabled_direct_source_count,
        available_disabled_global_source_count: coverage.available_disabled_global_source_count,
        enabled_category_counts: coverage.enabled_category_counts,
        direct_source_ids: coverage.direct_source_ids.into_iter().collect(),
        available_disabled_source_ids: coverage.available_disabled_source_ids.into_iter().collect(),
        available_disabled_direct_sources: coverage
            .available_disabled_direct_sources
            .into_values()
            .collect(),
        quality_gaps,
    }
}

fn collect_asset_coverage(registry: &SourceRegistry, asset: &str) -> AssetCoverage {
    let mut coverage = AssetCoverage::default();
    for source in registry
        .sources
        .iter()
        .filter(|source| source_applies_to_asset(source, asset))
    {
        coverage.record_source(source, asset);
    }
    coverage
}

use crate::coverage::report::AvailableDisabledSourceDetail;
use crate::coverage::rules::{quality_gaps, source_has_direct_asset};
use crate::registry::Source;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub(in crate::coverage) struct AssetCoverage {
    pub(in crate::coverage) enabled_category_counts: BTreeMap<String, usize>,
    pub(in crate::coverage) direct_source_ids: BTreeSet<String>,
    pub(in crate::coverage) available_disabled_source_ids: BTreeSet<String>,
    pub(in crate::coverage) available_disabled_direct_sources:
        BTreeMap<String, AvailableDisabledSourceDetail>,
    pub(in crate::coverage) enabled_direct_source_count: usize,
    pub(in crate::coverage) enabled_global_source_count: usize,
    pub(in crate::coverage) available_disabled_direct_source_count: usize,
    pub(in crate::coverage) available_disabled_global_source_count: usize,
}

impl AssetCoverage {
    pub(in crate::coverage) fn record_source(&mut self, source: &Source, asset: &str) {
        if source.enabled {
            self.record_enabled_source(source, asset);
            return;
        }
        if source.source_state.as_deref() == Some("available_disabled") {
            self.record_available_disabled_source(source, asset);
        }
    }

    fn record_enabled_source(&mut self, source: &Source, asset: &str) {
        *self
            .enabled_category_counts
            .entry(source.source_category.clone())
            .or_insert(0) += 1;
        if source_has_direct_asset(source, asset) {
            self.enabled_direct_source_count += 1;
            self.direct_source_ids.insert(source.source_id.clone());
        } else {
            self.enabled_global_source_count += 1;
        }
    }

    fn record_available_disabled_source(&mut self, source: &Source, asset: &str) {
        if source_has_direct_asset(source, asset) {
            self.available_disabled_direct_source_count += 1;
            self.available_disabled_direct_sources.insert(
                source.source_id.clone(),
                AvailableDisabledSourceDetail::from_source(source),
            );
        } else {
            self.available_disabled_global_source_count += 1;
        }
        self.available_disabled_source_ids
            .insert(source.source_id.clone());
    }

    pub(in crate::coverage) fn enabled_source_count(&self) -> usize {
        self.enabled_direct_source_count + self.enabled_global_source_count
    }

    fn available_disabled_source_count(&self) -> usize {
        self.available_disabled_direct_source_count + self.available_disabled_global_source_count
    }

    pub(in crate::coverage) fn quality_gaps(&self) -> Vec<String> {
        quality_gaps(
            &self.enabled_category_counts,
            self.enabled_direct_source_count,
            self.available_disabled_source_count(),
        )
    }
}

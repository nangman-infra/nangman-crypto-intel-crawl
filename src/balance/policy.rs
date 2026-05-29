use super::classification::{is_community_source, is_derivatives_snapshot_source};
use crate::registry::Source;

const DERIVATIVES_SAFETY_MAX_EVENTS_PER_RUN: usize = 12;
const DERIVATIVES_SAFETY_MAX_EVENTS_PER_SOURCE: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceBalancePolicy {
    pub(crate) derivatives_max_events_per_run: usize,
    pub(crate) derivatives_max_events_per_source: usize,
    pub(crate) community_max_events_per_run: usize,
    pub(crate) community_max_events_per_source: usize,
}

impl SourceBalancePolicy {
    pub(crate) fn effective_derivatives_max_events_per_run(&self) -> usize {
        self.derivatives_max_events_per_run
            .min(DERIVATIVES_SAFETY_MAX_EVENTS_PER_RUN)
    }

    pub(crate) fn effective_derivatives_max_events_per_source(&self) -> usize {
        self.derivatives_max_events_per_source
            .min(DERIVATIVES_SAFETY_MAX_EVENTS_PER_SOURCE)
    }

    pub(crate) fn effective_item_limit(&self, source: &Source, requested_limit: usize) -> usize {
        if is_derivatives_snapshot_source(source) {
            requested_limit.min(self.effective_derivatives_max_events_per_source())
        } else if is_community_source(source) {
            requested_limit.min(self.community_max_events_per_source)
        } else {
            requested_limit
        }
    }
}

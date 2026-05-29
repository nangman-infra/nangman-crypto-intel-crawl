use super::SourceBalancePolicy;
use super::classification::{is_community_source, is_derivatives_snapshot_source};
use crate::registry::Source;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(crate) struct SourceBalanceTracker {
    derivatives_emitted: usize,
    community_emitted: usize,
    category_emitted: BTreeMap<String, usize>,
}

impl SourceBalanceTracker {
    pub(crate) fn admit(&mut self, source: &Source, policy: SourceBalancePolicy) -> Admission {
        if is_derivatives_snapshot_source(source) {
            if self.derivatives_emitted >= policy.effective_derivatives_max_events_per_run() {
                return Admission::Suppress {
                    reason: "derivatives_snapshot_run_cap".to_owned(),
                };
            }
            self.derivatives_emitted += 1;
        } else if is_community_source(source) {
            if self.community_emitted >= policy.community_max_events_per_run {
                return Admission::Suppress {
                    reason: "community_reaction_run_cap".to_owned(),
                };
            }
            self.community_emitted += 1;
        }
        *self
            .category_emitted
            .entry(source.source_category.clone())
            .or_insert(0) += 1;
        Admission::Admit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Admission {
    Admit,
    Suppress { reason: String },
}

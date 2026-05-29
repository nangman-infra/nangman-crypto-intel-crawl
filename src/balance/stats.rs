use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceRunStats {
    pub(crate) items_seen: usize,
    pub(crate) candidates_after_dedup: usize,
    pub(crate) events_emitted: usize,
    pub(crate) duplicates_skipped: usize,
    pub(crate) suppressed_by_balance: usize,
    pub(crate) suppression_reasons: BTreeMap<String, usize>,
}

impl SourceRunStats {
    pub(crate) fn record_suppression(&mut self, reason: String) {
        self.suppressed_by_balance += 1;
        *self.suppression_reasons.entry(reason).or_insert(0) += 1;
    }
}

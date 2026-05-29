use crate::balance::{SourceBalanceRecord, SourceBalanceTracker};
use crate::dedup::DedupStore;
use crate::event::RawIntelEvent;
use crate::health::{SourceHealRecord, SourceHealthRecord};

pub(crate) struct CrawlBuffers<'a> {
    pub(in crate::crawl_loop) dedup: &'a mut DedupStore,
    pub(crate) raw_events: Vec<RawIntelEvent>,
    pub(crate) health_records: Vec<SourceHealthRecord>,
    pub(crate) heal_records: Vec<SourceHealRecord>,
    pub(crate) balance_records: Vec<SourceBalanceRecord>,
    pub(in crate::crawl_loop) balance_tracker: SourceBalanceTracker,
}

impl<'a> CrawlBuffers<'a> {
    pub(in crate::crawl_loop) fn new(dedup: &'a mut DedupStore) -> Self {
        Self {
            dedup,
            raw_events: Vec::new(),
            health_records: Vec::new(),
            heal_records: Vec::new(),
            balance_records: Vec::new(),
            balance_tracker: SourceBalanceTracker::default(),
        }
    }
}

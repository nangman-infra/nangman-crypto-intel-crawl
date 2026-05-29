use serde::Serialize;

#[derive(Debug, Default)]
pub(in crate::crawl_loop) struct SourceOutcome {
    pub(in crate::crawl_loop) events_written: usize,
    pub(in crate::crawl_loop) duplicates_skipped: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct CrawlSummary {
    pub(crate) dry_run: bool,
    pub(crate) nats_publish_enabled: bool,
    pub(crate) sources_selected: usize,
    pub(crate) sources_ok: usize,
    pub(crate) sources_failed: usize,
    pub(crate) items_seen: usize,
    pub(crate) events_written: usize,
    pub(crate) events_published: usize,
    pub(crate) pointer_publish_pending: usize,
    pub(crate) events_skipped_duplicate: usize,
    pub(crate) events_suppressed_by_balance: usize,
    pub(crate) source_health_written: usize,
    pub(crate) source_heal_written: usize,
    pub(crate) source_coverage_written: usize,
    pub(crate) source_balance_written: usize,
    pub(crate) outbox_published_written: usize,
    pub(crate) outbox_pending_written: usize,
    pub(crate) manifest_written: usize,
    pub(crate) source_coverage_key: Option<String>,
    pub(crate) source_balance_key: Option<String>,
    pub(crate) outbox_published_key: Option<String>,
    pub(crate) outbox_pending_key: Option<String>,
    pub(crate) manifest_key: Option<String>,
    pub(in crate::crawl_loop) failures: Vec<SourceFailure>,
}

impl CrawlSummary {
    pub(in crate::crawl_loop) fn new(
        sources_selected: usize,
        dry_run: bool,
        nats_publish_enabled: bool,
    ) -> Self {
        Self {
            dry_run,
            nats_publish_enabled,
            sources_selected,
            sources_ok: 0,
            sources_failed: 0,
            items_seen: 0,
            events_written: 0,
            events_published: 0,
            pointer_publish_pending: 0,
            events_skipped_duplicate: 0,
            events_suppressed_by_balance: 0,
            source_health_written: 0,
            source_heal_written: 0,
            source_coverage_written: 0,
            source_balance_written: 0,
            outbox_published_written: 0,
            outbox_pending_written: 0,
            manifest_written: 0,
            source_coverage_key: None,
            source_balance_key: None,
            outbox_published_key: None,
            outbox_pending_key: None,
            manifest_key: None,
            failures: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(in crate::crawl_loop) struct SourceFailure {
    pub(in crate::crawl_loop) source_id: String,
    pub(in crate::crawl_loop) error: String,
}

use super::{
    DEFAULT_CHUNK_MAX_RECORDS, DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN,
    DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE, DEFAULT_DEDUP_LOOKBACK_DAYS,
    DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN, DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE,
    DEFAULT_NATS_STREAM, DEFAULT_NATS_SUBJECT, DEFAULT_OBJECT_STORE_BUCKET,
    DEFAULT_OBJECT_STORE_REGION, DEFAULT_SOURCE_REGISTRY_PATH,
};
use crate::object_store::ObjectStoreConfig;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Args {
    pub(crate) source_registry: PathBuf,
    pub(crate) max_items_per_source: usize,
    pub(crate) schedule_interval_ms: Option<u64>,
    pub(crate) dry_run: bool,
    pub(crate) source_id: Option<String>,
    pub(crate) nats_url: Option<String>,
    pub(crate) nats_subject: String,
    pub(crate) nats_stream: String,
    pub(crate) object_store: ObjectStoreConfig,
    pub(crate) dedup_lookback_days: u16,
    pub(crate) chunk_max_records: usize,
    pub(crate) derivatives_max_events_per_run: usize,
    pub(crate) derivatives_max_events_per_source: usize,
    pub(crate) community_max_events_per_run: usize,
    pub(crate) community_max_events_per_source: usize,
    pub(crate) backfill_start_ms: Option<i64>,
    pub(crate) backfill_end_ms: Option<i64>,
    pub(crate) replay_pending_outbox: bool,
}

impl Args {
    pub(super) fn with_defaults() -> Self {
        Self {
            source_registry: PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH),
            max_items_per_source: 50,
            schedule_interval_ms: None,
            dry_run: false,
            source_id: None,
            nats_url: None,
            nats_subject: DEFAULT_NATS_SUBJECT.to_owned(),
            nats_stream: DEFAULT_NATS_STREAM.to_owned(),
            object_store: ObjectStoreConfig {
                bucket: DEFAULT_OBJECT_STORE_BUCKET.to_owned(),
                region: DEFAULT_OBJECT_STORE_REGION.to_owned(),
            },
            dedup_lookback_days: DEFAULT_DEDUP_LOOKBACK_DAYS,
            chunk_max_records: DEFAULT_CHUNK_MAX_RECORDS,
            derivatives_max_events_per_run: DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN,
            derivatives_max_events_per_source: DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE,
            community_max_events_per_run: DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN,
            community_max_events_per_source: DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE,
            backfill_start_ms: None,
            backfill_end_ms: None,
            replay_pending_outbox: false,
        }
    }
}

pub(crate) const DEFAULT_SOURCE_REGISTRY_PATH: &str =
    "/opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json";
pub(crate) const DEFAULT_NATS_SUBJECT: &str = "raw_intel_event.created";
pub(crate) const DEFAULT_NATS_STREAM: &str = "RAW_INTEL";
pub(crate) const DEFAULT_OBJECT_STORE_BUCKET: &str = "<bucket-name>";
pub(crate) const DEFAULT_OBJECT_STORE_REGION: &str = "ap-northeast-2";
pub(crate) const DEFAULT_DEDUP_LOOKBACK_DAYS: u16 = 14;
pub(crate) const DEFAULT_CHUNK_MAX_RECORDS: usize = 1000;
pub(crate) const DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN: usize = 12;
pub(crate) const DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE: usize = 6;
pub(crate) const DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN: usize = 30;
pub(crate) const DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE: usize = 5;

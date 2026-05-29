use super::{
    DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN, DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE,
    DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN, DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE,
    DEFAULT_NATS_STREAM, DEFAULT_NATS_SUBJECT, DEFAULT_OBJECT_STORE_BUCKET,
    DEFAULT_OBJECT_STORE_REGION, DEFAULT_SOURCE_REGISTRY_PATH,
};

pub(super) fn help() -> String {
    format!(
        "Usage: intel-crawl-app [--source-registry ABS_PATH] [--max-items-per-source N] [--schedule-interval-ms N] [--source-id ID] [--nats-url nats://HOST:4222] [--nats-subject SUBJECT] [--nats-stream STREAM] [--object-store-bucket BUCKET] [--object-store-region REGION] [--dedup-lookback-days N] [--chunk-max-records N] [--derivatives-max-events-per-run N] [--derivatives-max-events-per-source N] [--community-max-events-per-run N] [--community-max-events-per-source N] [--backfill-start-ms TS] [--backfill-end-ms TS] [--replay-pending-outbox] [--dry-run]\n\nDefaults:\n  --source-registry {DEFAULT_SOURCE_REGISTRY_PATH}\n  --nats-subject {DEFAULT_NATS_SUBJECT}\n  --nats-stream {DEFAULT_NATS_STREAM}\n  --object-store-bucket {DEFAULT_OBJECT_STORE_BUCKET}\n  --object-store-region {DEFAULT_OBJECT_STORE_REGION}\n  --derivatives-max-events-per-run {DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN}\n  --derivatives-max-events-per-source {DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE}\n  --community-max-events-per-run {DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN}\n  --community-max-events-per-source {DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE}"
    )
}

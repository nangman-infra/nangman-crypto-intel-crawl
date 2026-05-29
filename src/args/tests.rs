use super::*;
use std::path::PathBuf;

#[test]
fn parses_defaults() {
    let args = Args::parse(["intel-crawl-app".to_owned()].into_iter()).unwrap();

    assert_eq!(
        args.source_registry,
        PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH)
    );
    assert_eq!(args.max_items_per_source, 50);
    assert_eq!(args.nats_subject, DEFAULT_NATS_SUBJECT);
    assert_eq!(args.nats_stream, DEFAULT_NATS_STREAM);
    assert_eq!(args.object_store.bucket, DEFAULT_OBJECT_STORE_BUCKET);
    assert_eq!(args.object_store.region, DEFAULT_OBJECT_STORE_REGION);
    assert_eq!(
        args.derivatives_max_events_per_run,
        DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN
    );
    assert_eq!(
        args.derivatives_max_events_per_source,
        DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE
    );
    assert_eq!(
        args.community_max_events_per_run,
        DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN
    );
    assert_eq!(
        args.community_max_events_per_source,
        DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE
    );
    assert_eq!(args.backfill_start_ms, None);
    assert_eq!(args.backfill_end_ms, None);
    assert!(!args.dry_run);
}

#[test]
fn rejects_relative_source_registry_path() {
    let error = Args::parse(
        [
            "intel-crawl-app".to_owned(),
            "--source-registry".to_owned(),
            "relative.json".to_owned(),
        ]
        .into_iter(),
    )
    .unwrap_err();

    assert!(error.contains("--source-registry requires an absolute path"));
}

#[test]
fn parses_nats_and_schedule_args() {
    let args = Args::parse(
        [
            "intel-crawl-app".to_owned(),
            "--nats-url".to_owned(),
            "nats://nats:4222".to_owned(),
            "--nats-subject".to_owned(),
            "raw_intel_event.created".to_owned(),
            "--nats-stream".to_owned(),
            "RAW_INTEL".to_owned(),
            "--schedule-interval-ms".to_owned(),
            "60000".to_owned(),
        ]
        .into_iter(),
    )
    .unwrap();

    assert_eq!(args.nats_url.as_deref(), Some("nats://nats:4222"));
    assert_eq!(args.nats_subject, "raw_intel_event.created");
    assert_eq!(args.nats_stream, "RAW_INTEL");
    assert_eq!(args.schedule_interval_ms, Some(60000));
}

#[test]
fn parses_object_store_args() {
    let args = Args::parse(
        [
            "intel-crawl-app".to_owned(),
            "--object-store-bucket".to_owned(),
            "nangman-crypto-dev-intel-crawl-l0-000000".to_owned(),
            "--object-store-region".to_owned(),
            "ap-northeast-2".to_owned(),
            "--dedup-lookback-days".to_owned(),
            "30".to_owned(),
            "--chunk-max-records".to_owned(),
            "500".to_owned(),
            "--derivatives-max-events-per-run".to_owned(),
            "25".to_owned(),
            "--derivatives-max-events-per-source".to_owned(),
            "10".to_owned(),
            "--community-max-events-per-run".to_owned(),
            "15".to_owned(),
            "--community-max-events-per-source".to_owned(),
            "3".to_owned(),
            "--backfill-start-ms".to_owned(),
            "1764892800000".to_owned(),
            "--backfill-end-ms".to_owned(),
            "1764979200000".to_owned(),
        ]
        .into_iter(),
    )
    .unwrap();

    assert_eq!(
        args.object_store.bucket,
        "nangman-crypto-dev-intel-crawl-l0-000000"
    );
    assert_eq!(args.object_store.region, "ap-northeast-2");
    assert_eq!(args.dedup_lookback_days, 30);
    assert_eq!(args.chunk_max_records, 500);
    assert_eq!(args.derivatives_max_events_per_run, 25);
    assert_eq!(args.derivatives_max_events_per_source, 10);
    assert_eq!(args.community_max_events_per_run, 15);
    assert_eq!(args.community_max_events_per_source, 3);
    assert_eq!(args.backfill_start_ms, Some(1764892800000));
    assert_eq!(args.backfill_end_ms, Some(1764979200000));
}

#[test]
fn rejects_incomplete_backfill_window() {
    let error = Args::parse(
        [
            "intel-crawl-app".to_owned(),
            "--backfill-start-ms".to_owned(),
            "1764892800000".to_owned(),
        ]
        .into_iter(),
    )
    .unwrap_err();

    assert!(error.contains("--backfill-end-ms is required"));
}

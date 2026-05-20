use crate::object_store::ObjectStoreConfig;
use std::path::PathBuf;

pub(crate) const DEFAULT_SOURCE_REGISTRY_PATH: &str =
    "/opt/nangman-crypto/intel-crawl/config/source-registry.rss-seed.v1.json";
pub(crate) const DEFAULT_NATS_SUBJECT: &str = "raw_intel_event.created";
pub(crate) const DEFAULT_NATS_STREAM: &str = "RAW_INTEL";
pub(crate) const DEFAULT_OBJECT_STORE_ENDPOINT: &str = "https://s3.nangman.cloud";
pub(crate) const DEFAULT_OBJECT_STORE_BUCKET: &str = "intel-crawl-app-l0";
pub(crate) const DEFAULT_OBJECT_STORE_REGION: &str = "us-east-1";
pub(crate) const DEFAULT_DEDUP_LOOKBACK_DAYS: u16 = 14;
pub(crate) const DEFAULT_CHUNK_MAX_RECORDS: usize = 1000;
pub(crate) const DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN: usize = 12;
pub(crate) const DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE: usize = 6;
pub(crate) const DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN: usize = 30;
pub(crate) const DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE: usize = 5;

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
    pub(crate) fn parse<I>(mut values: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let _program = values.next();
        let mut args = default_args();

        while let Some(arg) = values.next() {
            apply_arg(&mut args, arg.as_str(), &mut values)?;
        }

        validate_backfill_window(&args)?;
        Ok(args)
    }
}

fn default_args() -> Args {
    Args {
        source_registry: PathBuf::from(DEFAULT_SOURCE_REGISTRY_PATH),
        max_items_per_source: 50,
        schedule_interval_ms: None,
        dry_run: false,
        source_id: None,
        nats_url: None,
        nats_subject: DEFAULT_NATS_SUBJECT.to_owned(),
        nats_stream: DEFAULT_NATS_STREAM.to_owned(),
        object_store: ObjectStoreConfig {
            endpoint: DEFAULT_OBJECT_STORE_ENDPOINT.to_owned(),
            bucket: DEFAULT_OBJECT_STORE_BUCKET.to_owned(),
            region: DEFAULT_OBJECT_STORE_REGION.to_owned(),
            force_path_style: true,
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

fn apply_arg<I>(args: &mut Args, arg: &str, values: &mut I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    match arg {
        "--source-registry" => parse_source_registry(args, values.next()),
        "--max-items-per-source" => {
            args.max_items_per_source =
                positive_usize_arg(values.next(), "--max-items-per-source")?;
            Ok(())
        }
        "--schedule-interval-ms" => {
            args.schedule_interval_ms =
                Some(positive_u64_arg(values.next(), "--schedule-interval-ms")?);
            Ok(())
        }
        "--dry-run" => {
            args.dry_run = true;
            Ok(())
        }
        "--source-id" => parse_source_id(args, values.next()),
        "--nats-url" => parse_nats_url(args, values.next()),
        "--nats-subject" => parse_non_empty_token(
            &mut args.nats_subject,
            values.next(),
            "--nats-subject",
            "subject",
        ),
        "--nats-stream" => parse_non_empty_token(
            &mut args.nats_stream,
            values.next(),
            "--nats-stream",
            "stream name",
        ),
        "--object-store-endpoint" => parse_object_store_endpoint(args, values.next()),
        "--object-store-bucket" => parse_non_empty_token(
            &mut args.object_store.bucket,
            values.next(),
            "--object-store-bucket",
            "bucket",
        ),
        "--object-store-region" => parse_non_empty_token(
            &mut args.object_store.region,
            values.next(),
            "--object-store-region",
            "region",
        ),
        "--object-store-force-path-style" => {
            let value = values.next().ok_or_else(|| {
                "--object-store-force-path-style requires true or false".to_owned()
            })?;
            args.object_store.force_path_style = parse_bool(
                &value,
                "--object-store-force-path-style requires true or false",
            )?;
            Ok(())
        }
        "--dedup-lookback-days" => {
            args.dedup_lookback_days = positive_u16_arg(values.next(), "--dedup-lookback-days")?;
            Ok(())
        }
        "--chunk-max-records" => {
            args.chunk_max_records = positive_usize_arg(values.next(), "--chunk-max-records")?;
            Ok(())
        }
        "--derivatives-max-events-per-run" => {
            args.derivatives_max_events_per_run =
                positive_usize_arg(values.next(), "--derivatives-max-events-per-run")?;
            Ok(())
        }
        "--derivatives-max-events-per-source" => {
            args.derivatives_max_events_per_source =
                positive_usize_arg(values.next(), "--derivatives-max-events-per-source")?;
            Ok(())
        }
        "--community-max-events-per-run" => {
            args.community_max_events_per_run =
                positive_usize_arg(values.next(), "--community-max-events-per-run")?;
            Ok(())
        }
        "--community-max-events-per-source" => {
            args.community_max_events_per_source =
                positive_usize_arg(values.next(), "--community-max-events-per-source")?;
            Ok(())
        }
        "--backfill-start-ms" => {
            args.backfill_start_ms =
                Some(non_negative_i64_arg(values.next(), "--backfill-start-ms")?);
            Ok(())
        }
        "--backfill-end-ms" => {
            args.backfill_end_ms = Some(non_negative_i64_arg(values.next(), "--backfill-end-ms")?);
            Ok(())
        }
        "--replay-pending-outbox" => {
            args.replay_pending_outbox = true;
            Ok(())
        }
        "--help" | "-h" => Err(help()),
        other => Err(format!("unknown argument: {other}\n\n{}", help())),
    }
}

fn parse_source_registry(args: &mut Args, value: Option<String>) -> Result<(), String> {
    args.source_registry = absolute_path_arg(value, "--source-registry requires an absolute path")?;
    Ok(())
}

fn parse_source_id(args: &mut Args, value: Option<String>) -> Result<(), String> {
    let source_id = non_empty_value(value, "--source-id", "source id")?;
    args.source_id = Some(source_id);
    Ok(())
}

fn parse_nats_url(args: &mut Args, value: Option<String>) -> Result<(), String> {
    let nats_url = value.ok_or_else(|| "--nats-url requires a NATS server URL".to_owned())?;
    if !nats_url.starts_with("nats://") && !nats_url.starts_with("tls://") {
        return Err("--nats-url must start with nats:// or tls://".to_owned());
    }
    args.nats_url = Some(nats_url);
    Ok(())
}

fn parse_object_store_endpoint(args: &mut Args, value: Option<String>) -> Result<(), String> {
    let endpoint =
        value.ok_or_else(|| "--object-store-endpoint requires an endpoint URL".to_owned())?;
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        return Err("--object-store-endpoint must start with http:// or https://".to_owned());
    }
    args.object_store.endpoint = endpoint.trim_end_matches('/').to_owned();
    Ok(())
}

fn parse_non_empty_token(
    target: &mut String,
    value: Option<String>,
    flag: &str,
    label: &str,
) -> Result<(), String> {
    *target = non_empty_value(value, flag, label)?;
    if target.split_whitespace().count() > 1 {
        return Err(format!("{flag} must not be empty or contain whitespace"));
    }
    Ok(())
}

fn non_empty_value(value: Option<String>, flag: &str, label: &str) -> Result<String, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a {label}"))?;
    if value.trim().is_empty() {
        return Err(format!("{flag} must not be empty"));
    }
    Ok(value)
}

fn validate_backfill_window(args: &Args) -> Result<(), String> {
    match (args.backfill_start_ms, args.backfill_end_ms) {
        (Some(start_ms), Some(end_ms)) if start_ms >= end_ms => {
            Err("--backfill-start-ms must be less than --backfill-end-ms".to_owned())
        }
        (Some(_), None) => Err("--backfill-end-ms is required with --backfill-start-ms".to_owned()),
        (None, Some(_)) => Err("--backfill-start-ms is required with --backfill-end-ms".to_owned()),
        _ => Ok(()),
    }
}

fn absolute_path_arg(value: Option<String>, message: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value.ok_or_else(|| message.to_owned())?);
    if !path.is_absolute() {
        return Err(message.to_owned());
    }
    Ok(path)
}

fn parse_bool(value: &str, message: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(message.to_owned()),
    }
}

fn positive_usize_arg(value: Option<String>, name: &str) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

fn positive_u64_arg(value: Option<String>, name: &str) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

fn positive_u16_arg(value: Option<String>, name: &str) -> Result<u16, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<u16>()
        .map_err(|_| format!("{name} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

fn non_negative_i64_arg(value: Option<String>, name: &str) -> Result<i64, String> {
    let value = value.ok_or_else(|| format!("{name} requires a timestamp in ms"))?;
    let parsed = value
        .parse::<i64>()
        .map_err(|_| format!("{name} must be a non-negative integer"))?;
    if parsed < 0 {
        return Err(format!("{name} must be non-negative"));
    }
    Ok(parsed)
}

fn help() -> String {
    format!(
        "Usage: intel-crawl-app [--source-registry ABS_PATH] [--max-items-per-source N] [--schedule-interval-ms N] [--source-id ID] [--nats-url nats://HOST:4222] [--nats-subject SUBJECT] [--nats-stream STREAM] [--object-store-endpoint URL] [--object-store-bucket BUCKET] [--object-store-region REGION] [--object-store-force-path-style true|false] [--dedup-lookback-days N] [--chunk-max-records N] [--derivatives-max-events-per-run N] [--derivatives-max-events-per-source N] [--community-max-events-per-run N] [--community-max-events-per-source N] [--backfill-start-ms TS] [--backfill-end-ms TS] [--replay-pending-outbox] [--dry-run]\n\nDefaults:\n  --source-registry {DEFAULT_SOURCE_REGISTRY_PATH}\n  --nats-subject {DEFAULT_NATS_SUBJECT}\n  --nats-stream {DEFAULT_NATS_STREAM}\n  --object-store-endpoint {DEFAULT_OBJECT_STORE_ENDPOINT}\n  --object-store-bucket {DEFAULT_OBJECT_STORE_BUCKET}\n  --object-store-region {DEFAULT_OBJECT_STORE_REGION}\n  --derivatives-max-events-per-run {DEFAULT_DERIVATIVES_MAX_EVENTS_PER_RUN}\n  --derivatives-max-events-per-source {DEFAULT_DERIVATIVES_MAX_EVENTS_PER_SOURCE}\n  --community-max-events-per-run {DEFAULT_COMMUNITY_MAX_EVENTS_PER_RUN}\n  --community-max-events-per-source {DEFAULT_COMMUNITY_MAX_EVENTS_PER_SOURCE}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(args.object_store.endpoint, DEFAULT_OBJECT_STORE_ENDPOINT);
        assert_eq!(args.object_store.bucket, DEFAULT_OBJECT_STORE_BUCKET);
        assert_eq!(args.object_store.region, DEFAULT_OBJECT_STORE_REGION);
        assert!(args.object_store.force_path_style);
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
                "--object-store-endpoint".to_owned(),
                "https://s3.nangman.cloud/".to_owned(),
                "--object-store-bucket".to_owned(),
                "intel-crawl-app-l0".to_owned(),
                "--object-store-region".to_owned(),
                "us-east-1".to_owned(),
                "--object-store-force-path-style".to_owned(),
                "true".to_owned(),
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

        assert_eq!(args.object_store.endpoint, "https://s3.nangman.cloud");
        assert_eq!(args.object_store.bucket, "intel-crawl-app-l0");
        assert_eq!(args.object_store.region, "us-east-1");
        assert!(args.object_store.force_path_style);
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
}

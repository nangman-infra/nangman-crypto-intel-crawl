use super::help::help;
use super::types::Args;
use super::validation::{
    absolute_path_arg, non_empty_value, non_negative_i64_arg, positive_u16_arg, positive_u64_arg,
    positive_usize_arg, validate_backfill_window,
};

impl Args {
    pub(crate) fn parse<I>(mut values: I) -> Result<Self, String>
    where
        I: Iterator<Item = String>,
    {
        let _program = values.next();
        let mut args = Args::with_defaults();

        while let Some(arg) = values.next() {
            apply_arg(&mut args, arg.as_str(), &mut values)?;
        }

        validate_backfill_window(&args)?;
        Ok(args)
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

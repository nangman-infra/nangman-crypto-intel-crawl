use super::types::Args;
use std::path::PathBuf;

pub(super) fn non_empty_value(
    value: Option<String>,
    flag: &str,
    label: &str,
) -> Result<String, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a {label}"))?;
    if value.trim().is_empty() {
        return Err(format!("{flag} must not be empty"));
    }
    Ok(value)
}

pub(super) fn validate_backfill_window(args: &Args) -> Result<(), String> {
    match (args.backfill_start_ms, args.backfill_end_ms) {
        (Some(start_ms), Some(end_ms)) if start_ms >= end_ms => {
            Err("--backfill-start-ms must be less than --backfill-end-ms".to_owned())
        }
        (Some(_), None) => Err("--backfill-end-ms is required with --backfill-start-ms".to_owned()),
        (None, Some(_)) => Err("--backfill-start-ms is required with --backfill-end-ms".to_owned()),
        _ => Ok(()),
    }
}

pub(super) fn absolute_path_arg(value: Option<String>, message: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value.ok_or_else(|| message.to_owned())?);
    if !path.is_absolute() {
        return Err(message.to_owned());
    }
    Ok(path)
}

pub(super) fn positive_usize_arg(value: Option<String>, name: &str) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

pub(super) fn positive_u64_arg(value: Option<String>, name: &str) -> Result<u64, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

pub(super) fn positive_u16_arg(value: Option<String>, name: &str) -> Result<u16, String> {
    let value = value.ok_or_else(|| format!("{name} requires a number"))?;
    let parsed = value
        .parse::<u16>()
        .map_err(|_| format!("{name} must be a positive number"))?;
    if parsed == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(parsed)
}

pub(super) fn non_negative_i64_arg(value: Option<String>, name: &str) -> Result<i64, String> {
    let value = value.ok_or_else(|| format!("{name} requires a timestamp in ms"))?;
    let parsed = value
        .parse::<i64>()
        .map_err(|_| format!("{name} must be a non-negative integer"))?;
    if parsed < 0 {
        return Err(format!("{name} must be non-negative"));
    }
    Ok(parsed)
}

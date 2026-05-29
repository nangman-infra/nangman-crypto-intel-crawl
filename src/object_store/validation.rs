use std::error::Error;
use std::net::Ipv4Addr;

const MAX_OBJECT_KEY_BYTES: usize = 1024;
const RESERVED_BUCKET_PREFIXES: [&str; 3] = ["xn--", "sthree-", "amzn-s3-demo-"];
const RESERVED_BUCKET_SUFFIXES: [&str; 5] =
    ["-s3alias", "--ol-s3", ".mrap", "--x-s3", "--table-s3"];

pub(super) fn validate_bucket_name(bucket: &str) -> Result<(), Box<dyn Error>> {
    if bucket.trim().is_empty() {
        return Err("object store bucket is required".into());
    }
    if bucket != bucket.trim() {
        return Err("object store bucket must not have leading or trailing whitespace".into());
    }
    if bucket.contains('<') || bucket.contains('>') {
        return Err(
            "object store bucket must be a real AWS S3 bucket name, not a placeholder".into(),
        );
    }
    if !(3..=63).contains(&bucket.len()) {
        return Err("object store bucket must be between 3 and 63 characters".into());
    }
    if !bucket.chars().all(is_bucket_name_char) {
        return Err(
            "object store bucket can only contain lowercase letters, numbers, periods, and hyphens"
                .into(),
        );
    }
    if !bucket
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        || !bucket
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
    {
        return Err("object store bucket must begin and end with a letter or number".into());
    }
    if bucket.contains("..") {
        return Err("object store bucket must not contain adjacent periods".into());
    }
    if bucket.contains(".-") || bucket.contains("-.") {
        return Err("object store bucket labels must not begin or end with a hyphen".into());
    }
    if bucket.parse::<Ipv4Addr>().is_ok() {
        return Err("object store bucket must not be formatted as an IP address".into());
    }
    if RESERVED_BUCKET_PREFIXES
        .iter()
        .any(|prefix| bucket.starts_with(prefix))
    {
        return Err("object store bucket uses an AWS reserved prefix".into());
    }
    if RESERVED_BUCKET_SUFFIXES
        .iter()
        .any(|suffix| bucket.ends_with(suffix))
    {
        return Err("object store bucket uses an AWS reserved suffix".into());
    }
    Ok(())
}

pub(super) fn validate_region(region: &str) -> Result<(), Box<dyn Error>> {
    if region.trim().is_empty() {
        return Err("object store region is required".into());
    }
    if region != region.trim() || region.chars().any(char::is_control) {
        return Err("object store region must not contain whitespace or control characters".into());
    }
    Ok(())
}

pub(super) fn validate_object_key(key: &str) -> Result<(), Box<dyn Error>> {
    validate_object_name(key, "object store key", false)
}

pub(super) fn validate_object_prefix(prefix: &str) -> Result<(), Box<dyn Error>> {
    validate_object_name(prefix, "object store prefix", true)
}

fn validate_object_name(
    value: &str,
    label: &'static str,
    allow_empty: bool,
) -> Result<(), Box<dyn Error>> {
    if value.is_empty() {
        if allow_empty {
            return Ok(());
        }
        return Err(format!("{label} is required").into());
    }
    if value.len() > MAX_OBJECT_KEY_BYTES {
        return Err(format!("{label} must be at most {MAX_OBJECT_KEY_BYTES} bytes").into());
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{label} must not contain control characters").into());
    }
    if value
        .split('/')
        .any(|segment| matches!(segment, "." | ".."))
    {
        return Err(format!("{label} must not contain period-only path segments").into());
    }
    Ok(())
}

fn is_bucket_name_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-')
}

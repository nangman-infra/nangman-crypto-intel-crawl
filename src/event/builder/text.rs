use crate::normalization::hash_hex;
use chrono::{DateTime, Utc};

pub(super) fn parse_published_at_ms(value: &str) -> Option<i64> {
    if let Ok(timestamp_ms) = value.parse::<i64>() {
        return Some(timestamp_ms);
    }

    DateTime::parse_from_rfc2822(value)
        .or_else(|_| DateTime::parse_from_rfc3339(value))
        .map(|date| date.with_timezone(&Utc).timestamp_millis())
        .ok()
}

pub(super) fn clean_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn short_hash(value: &str) -> String {
    hash_hex(value).chars().take(24).collect()
}

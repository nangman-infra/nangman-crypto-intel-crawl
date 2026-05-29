use crate::event::RawIntelEvent;
use chrono::{Datelike, Duration, Utc};
use std::collections::BTreeSet;

pub(super) fn candidate_v2_prefixes(
    events: &[RawIntelEvent],
    lookback_days: u16,
) -> BTreeSet<String> {
    let hash_prefixes = events
        .iter()
        .flat_map(|event| {
            [
                hash_prefix(event.exact_source_key()),
                hash_prefix(event.canonical_url_hash()),
                hash_prefix(event.normalized_content_hash()),
            ]
        })
        .collect::<BTreeSet<_>>();
    let mut prefixes = BTreeSet::new();
    for date in recent_dates(lookback_days) {
        for hash_prefix in &hash_prefixes {
            prefixes.insert(format!(
                "dedup-index-v2/schema=dedup_index_v2/dt={date}/hash_prefix={hash_prefix}/"
            ));
        }
    }
    prefixes
}

fn hash_prefix(value: &str) -> String {
    value.chars().take(2).collect::<String>()
}

pub(super) fn parse_simhash(value: &str) -> Option<u64> {
    u64::from_str_radix(value, 16).ok()
}

pub(super) fn recent_dates(lookback_days: u16) -> Vec<String> {
    let today = Utc::now().date_naive();
    (0..=i64::from(lookback_days))
        .map(|days| {
            let date = today - Duration::days(days);
            format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
        })
        .collect()
}

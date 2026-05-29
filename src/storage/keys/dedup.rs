use crate::event::RawIntelEvent;

use super::path::{hash_prefix, path_segment};
use super::time::time_parts;
use super::{DEDUP_SCHEMA, DEDUP_V2_SCHEMA};

pub(in crate::storage) fn dedup_index_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "dedup-index/schema={DEDUP_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

pub(in crate::storage) fn dedup_index_v2_object_key(
    observed_at_ms: i64,
    run_id: &str,
    hash_prefix: &str,
) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "dedup-index-v2/schema={DEDUP_V2_SCHEMA}/dt={}/hash_prefix={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        path_segment(hash_prefix),
        parts.hour,
        path_segment(run_id)
    )
}

pub(in crate::storage) fn dedup_v2_hash_prefixes(event: &RawIntelEvent) -> Vec<String> {
    let mut prefixes = vec![
        hash_prefix(event.exact_source_key()),
        hash_prefix(event.canonical_url_hash()),
        hash_prefix(event.normalized_content_hash()),
    ];
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

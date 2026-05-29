use super::MANIFEST_SCHEMA;
use super::path::path_segment;
use super::time::time_parts;

pub(in crate::storage) fn manifest_object_key(started_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(started_at_ms);
    format!(
        "manifests/schema={MANIFEST_SCHEMA}/dt={}/hour={:02}/run_id={}.json",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

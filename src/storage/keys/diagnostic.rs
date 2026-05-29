use super::path::path_segment;
use super::time::time_parts;
use super::{
    POINTER_SCHEMA, SOURCE_BALANCE_SCHEMA, SOURCE_COVERAGE_SCHEMA, SOURCE_HEAL_SCHEMA,
    SOURCE_HEALTH_SCHEMA,
};

pub(in crate::storage) fn source_health_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-health/schema={SOURCE_HEALTH_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

pub(in crate::storage) fn source_heal_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-heal/schema={SOURCE_HEAL_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

pub(in crate::storage) fn source_coverage_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-coverage/schema={SOURCE_COVERAGE_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

pub(in crate::storage) fn source_balance_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-balance/schema={SOURCE_BALANCE_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

pub(in crate::storage) fn publish_outbox_object_key(
    status: &str,
    observed_at_ms: i64,
    run_id: &str,
) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "publish-outbox/status={}/schema={POINTER_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        path_segment(status),
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

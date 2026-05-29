use super::path::path_segment;
use super::{RAW_SCHEMA, RawPartition};

pub(in crate::storage) fn raw_object_key(
    partition: &RawPartition,
    run_id: &str,
    part_number: usize,
) -> String {
    format!(
        "raw-intel-events/schema={RAW_SCHEMA}/dt={}/hour={:02}/source_category={}/source_id={}/run_id={}/part-{part_number:06}.jsonl",
        partition.date,
        partition.hour,
        path_segment(&partition.source_category),
        path_segment(&partition.source_id),
        path_segment(run_id)
    )
}

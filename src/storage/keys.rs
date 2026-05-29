use crate::event::RawIntelEvent;

use super::model::{DEDUP_SCHEMA, DEDUP_V2_SCHEMA, MANIFEST_SCHEMA};

mod dedup;
mod diagnostic;
mod manifest;
mod path;
mod raw;
mod time;

#[cfg(test)]
mod tests;

pub(super) use dedup::{dedup_index_object_key, dedup_index_v2_object_key, dedup_v2_hash_prefixes};
pub(super) use diagnostic::{
    publish_outbox_object_key, source_balance_object_key, source_coverage_object_key,
    source_heal_object_key, source_health_object_key,
};
pub(super) use manifest::manifest_object_key;
pub(super) use raw::raw_object_key;

const RAW_SCHEMA: &str = "raw_intel_event_v1";
const SOURCE_HEALTH_SCHEMA: &str = "source_health_v1";
const SOURCE_HEAL_SCHEMA: &str = "source_heal_event_v1";
const POINTER_SCHEMA: &str = "raw_intel_event_created_v2";
const SOURCE_COVERAGE_SCHEMA: &str = "source_coverage_v1";
const SOURCE_BALANCE_SCHEMA: &str = "source_balance_v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RawPartition {
    pub(super) date: String,
    pub(super) hour: u32,
    pub(super) source_category: String,
    pub(super) source_id: String,
}

impl RawPartition {
    pub(super) fn from_event(event: &RawIntelEvent) -> Self {
        let parts = time::time_parts(event.fetched_at_ms());
        Self {
            date: parts.date,
            hour: parts.hour,
            source_category: event.source_category().to_owned(),
            source_id: event.source_id().to_owned(),
        }
    }
}

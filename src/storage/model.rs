use crate::event::{RawIntelEvent, RawIntelEventStorageRef};
use serde::Serialize;

pub(crate) const MANIFEST_SCHEMA: &str = "intel_l0_manifest_v1";
pub(crate) const DEDUP_SCHEMA: &str = "dedup_index_v1";
pub(crate) const DEDUP_V2_SCHEMA: &str = "dedup_index_v2";

#[derive(Debug, Clone)]
pub(crate) struct StoredRawIntelEvent {
    pub(crate) event: RawIntelEvent,
    pub(crate) storage_ref: RawIntelEventStorageRef,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UploadedObject {
    object_family: String,
    key: String,
    record_count: usize,
    byte_count: usize,
}

impl UploadedObject {
    pub(crate) fn new(
        object_family: impl Into<String>,
        key: impl Into<String>,
        record_count: usize,
        byte_count: usize,
    ) -> Self {
        Self {
            object_family: object_family.into(),
            key: key.into(),
            record_count,
            byte_count,
        }
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DedupIndexRecord {
    pub(crate) schema_version: String,
    pub(crate) dedup_key: String,
    pub(crate) event_id: String,
    pub(crate) source_id: String,
    pub(crate) content_hash: String,
    pub(crate) observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DedupIndexV2Record {
    pub(crate) schema_version: String,
    pub(crate) dedup_key: String,
    pub(crate) event_id: String,
    pub(crate) source_id: String,
    pub(crate) content_hash: String,
    pub(crate) exact_source_key: String,
    pub(crate) canonical_url_hash: String,
    pub(crate) normalized_content_hash: String,
    pub(crate) simhash64: String,
    pub(crate) dedup_decision: String,
    pub(crate) duplicate_of_event_id: Option<String>,
    pub(crate) observed_at_ms: i64,
}

pub(crate) struct ManifestInput {
    pub(crate) status: String,
    pub(crate) started_at_ms: i64,
    pub(crate) finished_at_ms: i64,
    pub(crate) uploaded_objects: Vec<UploadedObject>,
    pub(crate) raw_event_count: usize,
    pub(crate) pointer_published_count: usize,
    pub(crate) pointer_pending_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct IntelL0Manifest {
    pub(crate) schema_version: String,
    pub(crate) run_id: String,
    pub(crate) status: String,
    pub(crate) started_at_ms: i64,
    pub(crate) finished_at_ms: i64,
    pub(crate) raw_event_count: usize,
    pub(crate) pointer_published_count: usize,
    pub(crate) pointer_pending_count: usize,
    pub(crate) uploaded_object_count: usize,
    pub(crate) uploaded_objects: Vec<UploadedObject>,
}

use crate::event::{RawIntelEvent, RawIntelEventStorageRef};
use crate::object_store::ObjectStore;
use chrono::{DateTime, Timelike, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;

const RAW_SCHEMA: &str = "raw_intel_event_v1";
const MANIFEST_SCHEMA: &str = "intel_l0_manifest_v1";
const SOURCE_HEALTH_SCHEMA: &str = "source_health_v1";
const SOURCE_HEAL_SCHEMA: &str = "source_heal_event_v1";
const DEDUP_SCHEMA: &str = "dedup_index_v1";
const DEDUP_V2_SCHEMA: &str = "dedup_index_v2";
const POINTER_SCHEMA: &str = "raw_intel_event_created_v2";
const SOURCE_COVERAGE_SCHEMA: &str = "source_coverage_v1";
const SOURCE_BALANCE_SCHEMA: &str = "source_balance_v1";

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
    pub(crate) fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DedupIndexRecord {
    schema_version: String,
    dedup_key: String,
    event_id: String,
    source_id: String,
    content_hash: String,
    observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DedupIndexV2Record {
    schema_version: String,
    dedup_key: String,
    event_id: String,
    source_id: String,
    content_hash: String,
    exact_source_key: String,
    canonical_url_hash: String,
    normalized_content_hash: String,
    simhash64: String,
    dedup_decision: String,
    duplicate_of_event_id: Option<String>,
    observed_at_ms: i64,
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

pub(crate) struct IntelL0Storage {
    object_store: ObjectStore,
    run_id: String,
    chunk_max_records: usize,
}

impl IntelL0Storage {
    pub(crate) fn new(object_store: ObjectStore, run_id: String, chunk_max_records: usize) -> Self {
        Self {
            object_store,
            run_id,
            chunk_max_records: chunk_max_records.max(1),
        }
    }

    pub(crate) async fn write_raw_events(
        &self,
        events: &[RawIntelEvent],
    ) -> Result<(Vec<StoredRawIntelEvent>, Vec<UploadedObject>), Box<dyn Error>> {
        let mut stored = Vec::new();
        let mut uploaded = Vec::new();
        let mut grouped: BTreeMap<RawPartition, Vec<RawIntelEvent>> = BTreeMap::new();
        for event in events {
            grouped
                .entry(RawPartition::from_event(event))
                .or_default()
                .push(event.clone());
        }

        for (partition, records) in grouped {
            for (index, chunk) in records.chunks(self.chunk_max_records).enumerate() {
                let part_number = index + 1;
                let key = raw_object_key(&partition, &self.run_id, part_number);
                let (bytes, locators) = build_jsonl_chunk(chunk)?;
                let byte_count = bytes.len();
                self.object_store
                    .put_bytes(&key, bytes, "application/x-ndjson")
                    .await?;
                uploaded.push(UploadedObject {
                    object_family: "raw_intel_event".to_owned(),
                    key: key.clone(),
                    record_count: chunk.len(),
                    byte_count,
                });
                for (event, locator) in chunk.iter().cloned().zip(locators) {
                    let storage_ref = RawIntelEventStorageRef::aws_s3_jsonl_record(
                        self.object_store.bucket().to_owned(),
                        key.clone(),
                        locator.line_number,
                        locator.byte_offset,
                        locator.byte_length,
                        locator.content_sha256,
                    );
                    stored.push(StoredRawIntelEvent { event, storage_ref });
                }
            }
        }

        Ok((stored, uploaded))
    }

    pub(crate) async fn write_source_health<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_health",
            &source_health_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_source_heal<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_heal",
            &source_heal_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_source_coverage<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_coverage",
            &source_coverage_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_source_balance<T: Serialize>(
        &self,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "source_balance",
            &source_balance_object_key(observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_dedup_index(
        &self,
        events: &[RawIntelEvent],
        observed_at_ms: i64,
    ) -> Result<Vec<UploadedObject>, Box<dyn Error>> {
        let records = events
            .iter()
            .map(|event| DedupIndexRecord {
                schema_version: DEDUP_SCHEMA.to_owned(),
                dedup_key: event.dedup_key().to_owned(),
                event_id: event.event_id().to_owned(),
                source_id: event.source_id().to_owned(),
                content_hash: event.content_hash().to_owned(),
                observed_at_ms,
            })
            .collect::<Vec<_>>();
        let mut uploaded = Vec::new();
        if let Some(object) = self
            .write_single_jsonl_object(
                "dedup_index",
                &dedup_index_object_key(observed_at_ms, &self.run_id),
                &records,
            )
            .await?
        {
            uploaded.push(object);
        }
        uploaded.extend(self.write_dedup_index_v2(events, observed_at_ms).await?);
        Ok(uploaded)
    }

    async fn write_dedup_index_v2(
        &self,
        events: &[RawIntelEvent],
        observed_at_ms: i64,
    ) -> Result<Vec<UploadedObject>, Box<dyn Error>> {
        let mut grouped: BTreeMap<String, Vec<DedupIndexV2Record>> = BTreeMap::new();
        for event in events {
            let record = DedupIndexV2Record {
                schema_version: DEDUP_V2_SCHEMA.to_owned(),
                dedup_key: event.dedup_key().to_owned(),
                event_id: event.event_id().to_owned(),
                source_id: event.source_id().to_owned(),
                content_hash: event.content_hash().to_owned(),
                exact_source_key: event.exact_source_key().to_owned(),
                canonical_url_hash: event.canonical_url_hash().to_owned(),
                normalized_content_hash: event.normalized_content_hash().to_owned(),
                simhash64: format!("{:016x}", event.simhash64_value()),
                dedup_decision: event.dedup_decision().to_owned(),
                duplicate_of_event_id: event.duplicate_of_event_id().map(ToOwned::to_owned),
                observed_at_ms,
            };
            for hash_prefix in dedup_v2_hash_prefixes(event) {
                grouped.entry(hash_prefix).or_default().push(record.clone());
            }
        }

        let mut uploaded = Vec::new();
        for (hash_prefix, records) in grouped {
            let key = dedup_index_v2_object_key(observed_at_ms, &self.run_id, &hash_prefix);
            if let Some(object) = self
                .write_single_jsonl_object("dedup_index_v2", &key, &records)
                .await?
            {
                uploaded.push(object);
            }
        }
        Ok(uploaded)
    }

    pub(crate) async fn write_publish_outbox<T: Serialize>(
        &self,
        status: &str,
        records: &[T],
        observed_at_ms: i64,
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        self.write_single_jsonl_object(
            "publish_outbox",
            &publish_outbox_object_key(status, observed_at_ms, &self.run_id),
            records,
        )
        .await
    }

    pub(crate) async fn write_manifest(
        &self,
        input: ManifestInput,
    ) -> Result<UploadedObject, Box<dyn Error>> {
        let key = manifest_object_key(input.started_at_ms, &self.run_id);
        let manifest = IntelL0Manifest {
            schema_version: MANIFEST_SCHEMA.to_owned(),
            run_id: self.run_id.clone(),
            status: input.status,
            started_at_ms: input.started_at_ms,
            finished_at_ms: input.finished_at_ms,
            raw_event_count: input.raw_event_count,
            pointer_published_count: input.pointer_published_count,
            pointer_pending_count: input.pointer_pending_count,
            uploaded_object_count: input.uploaded_objects.len(),
            uploaded_objects: input.uploaded_objects,
        };
        let bytes = serde_json::to_vec_pretty(&manifest)?;
        let byte_count = bytes.len();
        self.object_store
            .put_bytes(&key, bytes, "application/json")
            .await?;
        Ok(UploadedObject {
            object_family: "manifest".to_owned(),
            key,
            record_count: 1,
            byte_count,
        })
    }

    async fn write_single_jsonl_object<T: Serialize>(
        &self,
        object_family: &str,
        key: &str,
        records: &[T],
    ) -> Result<Option<UploadedObject>, Box<dyn Error>> {
        if records.is_empty() {
            return Ok(None);
        }
        let (bytes, _) = build_jsonl_chunk(records)?;
        let byte_count = bytes.len();
        self.object_store
            .put_bytes(key, bytes, "application/x-ndjson")
            .await?;
        Ok(Some(UploadedObject {
            object_family: object_family.to_owned(),
            key: key.to_owned(),
            record_count: records.len(),
            byte_count,
        }))
    }
}

#[derive(Debug, Clone, Serialize)]
struct IntelL0Manifest {
    schema_version: String,
    run_id: String,
    status: String,
    started_at_ms: i64,
    finished_at_ms: i64,
    raw_event_count: usize,
    pointer_published_count: usize,
    pointer_pending_count: usize,
    uploaded_object_count: usize,
    uploaded_objects: Vec<UploadedObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RawPartition {
    date: String,
    hour: u32,
    source_category: String,
    source_id: String,
}

impl RawPartition {
    fn from_event(event: &RawIntelEvent) -> Self {
        let parts = time_parts(event.fetched_at_ms());
        Self {
            date: parts.date,
            hour: parts.hour,
            source_category: event.source_category().to_owned(),
            source_id: event.source_id().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonlRecordLocator {
    pub(crate) line_number: usize,
    pub(crate) byte_offset: usize,
    pub(crate) byte_length: usize,
    pub(crate) content_sha256: String,
}

pub(crate) fn build_jsonl_chunk<T: Serialize>(
    records: &[T],
) -> Result<(Vec<u8>, Vec<JsonlRecordLocator>), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let mut locators = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let line = serde_json::to_vec(record)?;
        let byte_offset = bytes.len();
        let byte_length = line.len();
        let content_sha256 = format!("sha256:{}", hash_bytes(&line));
        bytes.extend_from_slice(&line);
        bytes.push(b'\n');
        locators.push(JsonlRecordLocator {
            line_number: index + 1,
            byte_offset,
            byte_length,
            content_sha256,
        });
    }
    Ok((bytes, locators))
}

fn raw_object_key(partition: &RawPartition, run_id: &str, part_number: usize) -> String {
    format!(
        "raw-intel-events/schema={RAW_SCHEMA}/dt={}/hour={:02}/source_category={}/source_id={}/run_id={}/part-{part_number:06}.jsonl",
        partition.date,
        partition.hour,
        path_segment(&partition.source_category),
        path_segment(&partition.source_id),
        path_segment(run_id)
    )
}

fn source_health_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-health/schema={SOURCE_HEALTH_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

fn source_heal_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-heal/schema={SOURCE_HEAL_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

fn source_coverage_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-coverage/schema={SOURCE_COVERAGE_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

fn source_balance_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "source-balance/schema={SOURCE_BALANCE_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

fn dedup_index_object_key(observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "dedup-index/schema={DEDUP_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

fn dedup_index_v2_object_key(observed_at_ms: i64, run_id: &str, hash_prefix: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "dedup-index-v2/schema={DEDUP_V2_SCHEMA}/dt={}/hash_prefix={}/hour={:02}/run_id={}/part-000001.jsonl",
        parts.date,
        path_segment(hash_prefix),
        parts.hour,
        path_segment(run_id)
    )
}

fn publish_outbox_object_key(status: &str, observed_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(observed_at_ms);
    format!(
        "publish-outbox/status={}/schema={POINTER_SCHEMA}/dt={}/hour={:02}/run_id={}/part-000001.jsonl",
        path_segment(status),
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

fn manifest_object_key(started_at_ms: i64, run_id: &str) -> String {
    let parts = time_parts(started_at_ms);
    format!(
        "manifests/schema={MANIFEST_SCHEMA}/dt={}/hour={:02}/run_id={}.json",
        parts.date,
        parts.hour,
        path_segment(run_id)
    )
}

struct TimeParts {
    date: String,
    hour: u32,
}

fn time_parts(timestamp_ms: i64) -> TimeParts {
    let timestamp =
        DateTime::<Utc>::from_timestamp_millis(timestamp_ms).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    TimeParts {
        date: timestamp.format("%Y-%m-%d").to_string(),
        hour: timestamp.hour(),
    }
}

fn path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash_prefix(value: &str) -> String {
    value.chars().take(2).collect()
}

fn dedup_v2_hash_prefixes(event: &RawIntelEvent) -> Vec<String> {
    let mut prefixes = vec![
        hash_prefix(event.exact_source_key()),
        hash_prefix(event.canonical_url_hash()),
        hash_prefix(event.normalized_content_hash()),
    ];
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Record {
        id: &'static str,
    }

    #[test]
    fn jsonl_chunk_tracks_record_locations() {
        let records = vec![Record { id: "a" }, Record { id: "b" }];

        let (bytes, locators) = build_jsonl_chunk(&records).unwrap();

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\"id\":\"a\"}\n{\"id\":\"b\"}\n"
        );
        assert_eq!(locators[0].line_number, 1);
        assert_eq!(locators[0].byte_offset, 0);
        assert_eq!(locators[0].byte_length, "{\"id\":\"a\"}".len());
        assert_eq!(locators[1].line_number, 2);
        assert_eq!(locators[1].byte_offset, "{\"id\":\"a\"}\n".len());
    }

    #[test]
    fn raw_key_uses_partitioned_prefix() {
        let partition = RawPartition {
            date: "2026-05-07".to_owned(),
            hour: 19,
            source_category: "news".to_owned(),
            source_id: "coindesk_rss".to_owned(),
        };

        assert_eq!(
            raw_object_key(&partition, "intel-crawl-1", 2),
            "raw-intel-events/schema=raw_intel_event_v1/dt=2026-05-07/hour=19/source_category=news/source_id=coindesk_rss/run_id=intel-crawl-1/part-000002.jsonl"
        );
    }
}

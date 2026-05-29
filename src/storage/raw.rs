use super::IntelL0Storage;
use super::jsonl::build_jsonl_chunk;
use super::keys::{RawPartition, raw_object_key};
use super::model::{StoredRawIntelEvent, UploadedObject};
use crate::event::{RawIntelEvent, RawIntelEventStorageRef};
use std::collections::BTreeMap;
use std::error::Error;

impl IntelL0Storage {
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
                uploaded.push(UploadedObject::new(
                    "raw_intel_event",
                    key.clone(),
                    chunk.len(),
                    byte_count,
                ));
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
}

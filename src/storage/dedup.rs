use super::IntelL0Storage;
use super::keys::{dedup_index_object_key, dedup_index_v2_object_key, dedup_v2_hash_prefixes};
use super::model::{
    DEDUP_SCHEMA, DEDUP_V2_SCHEMA, DedupIndexRecord, DedupIndexV2Record, UploadedObject,
};
use crate::event::RawIntelEvent;
use std::collections::BTreeMap;
use std::error::Error;

impl IntelL0Storage {
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
}

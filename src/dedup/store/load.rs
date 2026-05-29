use super::{DedupStore, KnownEvent, KnownSimhash};
use crate::dedup::prefix::{candidate_v2_prefixes, parse_simhash, recent_dates};
use crate::event::RawIntelEvent;
use crate::object_store::ObjectStore;
use serde_json::Value;
use std::error::Error;

impl DedupStore {
    pub(crate) async fn load_from_object_store(
        object_store: &ObjectStore,
        lookback_days: u16,
    ) -> Result<Self, Box<dyn Error>> {
        let mut store = Self::default();
        for date in recent_dates(lookback_days) {
            let prefix = format!("dedup-index/schema=dedup_index_v1/dt={date}/");
            for key in object_store.list_keys(&prefix).await? {
                if !key.ends_with(".jsonl") || key.ends_with("_prefix.json") {
                    continue;
                }
                let raw = String::from_utf8(object_store.get_bytes(&key).await?)?;
                store.load_jsonl(&raw);
            }
        }
        Ok(store)
    }

    pub(crate) async fn load_candidate_shards(
        &mut self,
        object_store: &ObjectStore,
        events: &[RawIntelEvent],
        lookback_days: u16,
    ) -> Result<(), Box<dyn Error>> {
        for prefix in candidate_v2_prefixes(events, lookback_days) {
            if !self.loaded_v2_prefixes.insert(prefix.clone()) {
                continue;
            }
            for key in object_store.list_keys(&prefix).await? {
                if !key.ends_with(".jsonl") {
                    continue;
                }
                let raw = String::from_utf8(object_store.get_bytes(&key).await?)?;
                self.load_jsonl(&raw);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn from_jsonl(raw: &str) -> Self {
        let mut store = Self::default();
        store.load_jsonl(raw);
        store
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.legacy_keys.len() + self.exact_source_keys.len() + self.normalized_content_hashes.len()
    }

    fn load_jsonl(&mut self, raw: &str) {
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            self.load_record(&value);
        }
    }

    fn load_record(&mut self, value: &Value) {
        if let Some(dedup_key) = value.get("dedup_key").and_then(Value::as_str) {
            self.legacy_keys.insert(dedup_key.to_owned());
        }
        let Some(event_id) = value.get("event_id").and_then(Value::as_str) else {
            return;
        };
        let normalized_content_hash = value
            .get("normalized_content_hash")
            .or_else(|| value.get("content_hash"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if let Some(exact_source_key) = value.get("exact_source_key").and_then(Value::as_str) {
            self.exact_source_keys.insert(
                exact_source_key.to_owned(),
                KnownEvent {
                    event_id: event_id.to_owned(),
                    normalized_content_hash: normalized_content_hash.clone(),
                },
            );
        }
        if let Some(canonical_url_hash) = value.get("canonical_url_hash").and_then(Value::as_str) {
            self.canonical_url_hashes
                .insert(canonical_url_hash.to_owned(), event_id.to_owned());
        }
        if !normalized_content_hash.is_empty() {
            self.normalized_content_hashes
                .insert(normalized_content_hash, event_id.to_owned());
        }
        if let Some(simhash64) = value
            .get("simhash64")
            .and_then(Value::as_str)
            .and_then(parse_simhash)
        {
            self.simhashes.push(KnownSimhash {
                event_id: event_id.to_owned(),
                simhash64,
            });
        }
    }
}

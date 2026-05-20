use crate::object_store::ObjectStore;
use chrono::{Datelike, Duration, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::error::Error;

#[derive(Debug, Default)]
pub(crate) struct DedupStore {
    keys: HashSet<String>,
}

impl DedupStore {
    pub(crate) async fn load_from_object_store(
        object_store: &ObjectStore,
        lookback_days: u16,
    ) -> Result<Self, Box<dyn Error>> {
        let mut keys = HashSet::new();
        for date in recent_dates(lookback_days) {
            let prefix = format!("dedup-index/schema=dedup_index_v1/dt={date}/");
            for key in object_store.list_keys(&prefix).await? {
                if !key.ends_with(".jsonl") || key.ends_with("_prefix.json") {
                    continue;
                }
                let raw = String::from_utf8(object_store.get_bytes(&key).await?)?;
                load_keys_from_jsonl(&raw, &mut keys);
            }
        }
        Ok(Self { keys })
    }

    #[cfg(test)]
    pub(crate) fn from_jsonl(raw: &str) -> Self {
        let mut keys = HashSet::new();
        load_keys_from_jsonl(raw, &mut keys);
        Self { keys }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }

    pub(crate) fn is_new(&mut self, dedup_key: &str) -> bool {
        self.keys.insert(dedup_key.to_owned())
    }
}

fn load_keys_from_jsonl(raw: &str, keys: &mut HashSet<String>) {
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(dedup_key) = value.get("dedup_key").and_then(Value::as_str) {
            keys.insert(dedup_key.to_owned());
        }
    }
}

fn recent_dates(lookback_days: u16) -> Vec<String> {
    let today = Utc::now().date_naive();
    (0..=i64::from(lookback_days))
        .map(|days| {
            let date = today - Duration::days(days);
            format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_duplicate_keys() {
        let mut store = DedupStore::default();

        assert!(store.is_new("a"));
        assert!(!store.is_new("a"));
    }

    #[test]
    fn loads_dedup_keys_from_jsonl() {
        let store =
            DedupStore::from_jsonl(r#"{"schema_version":"dedup_index_v1","dedup_key":"a"}"#);

        assert_eq!(store.len(), 1);
    }
}

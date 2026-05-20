use crate::event::RawIntelEvent;
use crate::normalization::hamming_distance;
use crate::object_store::ObjectStore;
use chrono::{Datelike, Duration, Utc};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::error::Error;

const NEAR_DUPLICATE_HAMMING_THRESHOLD: u32 = 4;

#[derive(Debug, Default)]
pub(crate) struct DedupStore {
    legacy_keys: HashSet<String>,
    exact_source_keys: HashMap<String, KnownEvent>,
    canonical_url_hashes: HashMap<String, String>,
    normalized_content_hashes: HashMap<String, String>,
    simhashes: Vec<KnownSimhash>,
    loaded_v2_prefixes: HashSet<String>,
}

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

    pub(crate) fn decide_and_insert(&mut self, event: &RawIntelEvent) -> DedupDecision {
        if self.legacy_keys.contains(event.dedup_key()) {
            return DedupDecision::ExactDuplicate {
                duplicate_of_event_id: None,
            };
        }

        if let Some(known) = self.exact_source_keys.get(event.exact_source_key()) {
            if known.normalized_content_hash == event.normalized_content_hash() {
                return DedupDecision::ExactDuplicate {
                    duplicate_of_event_id: Some(known.event_id.clone()),
                };
            }
            let duplicate_of_event_id = Some(known.event_id.clone());
            self.insert_event(event);
            return DedupDecision::UpdateOfExisting {
                duplicate_of_event_id,
            };
        }

        if let Some(event_id) = self
            .normalized_content_hashes
            .get(event.normalized_content_hash())
        {
            return DedupDecision::ContentDuplicate {
                duplicate_of_event_id: Some(event_id.clone()),
            };
        }

        if let Some(known) = self.near_duplicate(event.simhash64_value()) {
            return DedupDecision::NearDuplicate {
                duplicate_of_event_id: Some(known.event_id.clone()),
            };
        }

        self.insert_event(event);
        DedupDecision::New
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

    fn near_duplicate(&self, simhash64: u64) -> Option<&KnownSimhash> {
        if simhash64 == 0 {
            return None;
        }
        self.simhashes.iter().find(|known| {
            known.simhash64 != 0
                && hamming_distance(known.simhash64, simhash64) <= NEAR_DUPLICATE_HAMMING_THRESHOLD
        })
    }

    fn insert_event(&mut self, event: &RawIntelEvent) {
        self.legacy_keys.insert(event.dedup_key().to_owned());
        self.exact_source_keys.insert(
            event.exact_source_key().to_owned(),
            KnownEvent {
                event_id: event.event_id().to_owned(),
                normalized_content_hash: event.normalized_content_hash().to_owned(),
            },
        );
        self.canonical_url_hashes.insert(
            event.canonical_url_hash().to_owned(),
            event.event_id().to_owned(),
        );
        self.normalized_content_hashes.insert(
            event.normalized_content_hash().to_owned(),
            event.event_id().to_owned(),
        );
        self.simhashes.push(KnownSimhash {
            event_id: event.event_id().to_owned(),
            simhash64: event.simhash64_value(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DedupDecision {
    New,
    ExactDuplicate {
        duplicate_of_event_id: Option<String>,
    },
    ContentDuplicate {
        duplicate_of_event_id: Option<String>,
    },
    NearDuplicate {
        duplicate_of_event_id: Option<String>,
    },
    UpdateOfExisting {
        duplicate_of_event_id: Option<String>,
    },
}

impl DedupDecision {
    pub(crate) fn is_skipped_duplicate(&self) -> bool {
        matches!(
            self,
            Self::ExactDuplicate { .. }
                | Self::ContentDuplicate { .. }
                | Self::NearDuplicate { .. }
        )
    }

    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::ExactDuplicate { .. } => "exact_duplicate",
            Self::ContentDuplicate { .. } => "content_duplicate",
            Self::NearDuplicate { .. } => "near_duplicate",
            Self::UpdateOfExisting { .. } => "update_of_existing",
        }
    }

    pub(crate) fn duplicate_of_event_id(&self) -> Option<String> {
        match self {
            Self::New => None,
            Self::ExactDuplicate {
                duplicate_of_event_id,
            }
            | Self::ContentDuplicate {
                duplicate_of_event_id,
            }
            | Self::NearDuplicate {
                duplicate_of_event_id,
            }
            | Self::UpdateOfExisting {
                duplicate_of_event_id,
            } => duplicate_of_event_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct KnownEvent {
    event_id: String,
    normalized_content_hash: String,
}

#[derive(Debug, Clone)]
struct KnownSimhash {
    event_id: String,
    simhash64: u64,
}

fn candidate_v2_prefixes(events: &[RawIntelEvent], lookback_days: u16) -> BTreeSet<String> {
    let hash_prefixes = events
        .iter()
        .flat_map(|event| {
            [
                hash_prefix(event.exact_source_key()),
                hash_prefix(event.canonical_url_hash()),
                hash_prefix(event.normalized_content_hash()),
            ]
        })
        .collect::<BTreeSet<_>>();
    let mut prefixes = BTreeSet::new();
    for date in recent_dates(lookback_days) {
        for hash_prefix in &hash_prefixes {
            prefixes.insert(format!(
                "dedup-index-v2/schema=dedup_index_v2/dt={date}/hash_prefix={hash_prefix}/"
            ));
        }
    }
    prefixes
}

fn hash_prefix(value: &str) -> String {
    value.chars().take(2).collect::<String>()
}

fn parse_simhash(value: &str) -> Option<u64> {
    u64::from_str_radix(value, 16).ok()
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
        let event = test_event("https://example.com/a", "A title", "A body");

        assert_eq!(store.decide_and_insert(&event), DedupDecision::New);
        assert!(store.decide_and_insert(&event).is_skipped_duplicate());
    }

    #[test]
    fn loads_dedup_keys_from_jsonl() {
        let store =
            DedupStore::from_jsonl(r#"{"schema_version":"dedup_index_v1","dedup_key":"a"}"#);

        assert_eq!(store.len(), 1);
    }

    #[test]
    fn detects_cross_source_content_duplicates() {
        let mut store = DedupStore::default();
        let first = test_event(
            "https://example.com/a",
            "Binance lists TEST",
            "Trading starts",
        );
        let mut second = test_event(
            "https://other.example/news",
            "Binance lists TEST",
            "Trading starts",
        );

        assert_eq!(store.decide_and_insert(&first), DedupDecision::New);
        let decision = store.decide_and_insert(&second);
        second.set_dedup_outcome(decision.label(), decision.duplicate_of_event_id());

        assert!(decision.is_skipped_duplicate());
    }

    fn test_event(url: &str, title: &str, body: &str) -> RawIntelEvent {
        use crate::event::build_raw_intel_event;
        use crate::item::FeedItem;
        use crate::registry::{AppliesToAssets, Source};

        let source = Source {
            source_id: "news".to_owned(),
            source_category: "news".to_owned(),
            source_name: "News".to_owned(),
            source_url: "https://example.com/rss.xml".to_owned(),
            fetch_method: "rss".to_owned(),
            adapter: None,
            max_items_per_run: None,
            trust_tier: "T1".to_owned(),
            cadence_tier: "medium".to_owned(),
            language_hint: "en".to_owned(),
            enabled: true,
            source_state: None,
            activation_blocker: None,
            top50_relevance_mode: "symbol_alias_match".to_owned(),
            applies_to_assets: AppliesToAssets::All("all_major_50".to_owned()),
        };
        let item = FeedItem {
            id: None,
            title: title.to_owned(),
            body: body.to_owned(),
            url: url.to_owned(),
            author: None,
            published_at: None,
            historical_source_depth: None,
            backfill_window_start_ms: None,
            backfill_window_end_ms: None,
            source_time_range_verified: None,
        };
        build_raw_intel_event(&source, &item, &[], 1)
    }
}

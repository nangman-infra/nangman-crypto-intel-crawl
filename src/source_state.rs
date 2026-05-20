use crate::fetch::{CacheHeaders, FetchMetadata};
use crate::object_store::ObjectStore;
use crate::registry::Source;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;

const SOURCE_FETCH_STATE_SCHEMA: &str = "source_fetch_state_v1";
const BASE_BACKOFF_MS: i64 = 60_000;
const MAX_BACKOFF_MS: i64 = 30 * 60_000;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SourceFetchState {
    schema_version: String,
    source_id: String,
    state_id: String,
    etag: Option<String>,
    last_modified: Option<String>,
    last_http_status: Option<u16>,
    last_checked_at_ms: Option<i64>,
    last_success_at_ms: Option<i64>,
    unchanged_count: usize,
    failure_count: usize,
    backoff_until_ms: Option<i64>,
    last_error: Option<String>,
}

impl SourceFetchState {
    pub(crate) fn new(source: &Source) -> Self {
        Self {
            schema_version: SOURCE_FETCH_STATE_SCHEMA.to_owned(),
            source_id: source.source_id.clone(),
            state_id: state_id(&source.source_id),
            etag: None,
            last_modified: None,
            last_http_status: None,
            last_checked_at_ms: None,
            last_success_at_ms: None,
            unchanged_count: 0,
            failure_count: 0,
            backoff_until_ms: None,
            last_error: None,
        }
    }

    pub(crate) fn cache_headers(&self) -> CacheHeaders {
        CacheHeaders {
            etag: self.etag.clone(),
            last_modified: self.last_modified.clone(),
        }
    }

    pub(crate) fn backoff_until_ms(&self) -> Option<i64> {
        self.backoff_until_ms
    }

    pub(crate) fn is_backing_off(&self, now_ms: i64) -> bool {
        self.backoff_until_ms.is_some_and(|until| until > now_ms)
    }

    pub(crate) fn record_success(
        &mut self,
        metadata: &FetchMetadata,
        now_ms: i64,
        items_seen: usize,
    ) {
        self.last_http_status = Some(metadata.http_status);
        self.last_checked_at_ms = Some(now_ms);
        self.last_success_at_ms = Some(now_ms);
        self.failure_count = 0;
        self.backoff_until_ms = None;
        self.last_error = None;
        if metadata.etag.is_some() {
            self.etag = metadata.etag.clone();
        }
        if metadata.last_modified.is_some() {
            self.last_modified = metadata.last_modified.clone();
        }
        if items_seen > 0 {
            self.unchanged_count = 0;
        }
    }

    pub(crate) fn record_not_modified(&mut self, metadata: &FetchMetadata, now_ms: i64) {
        self.last_http_status = Some(metadata.http_status);
        self.last_checked_at_ms = Some(now_ms);
        self.last_success_at_ms = Some(now_ms);
        self.failure_count = 0;
        self.backoff_until_ms = None;
        self.last_error = None;
        self.unchanged_count += 1;
        if metadata.etag.is_some() {
            self.etag = metadata.etag.clone();
        }
        if metadata.last_modified.is_some() {
            self.last_modified = metadata.last_modified.clone();
        }
    }

    pub(crate) fn record_failure(&mut self, error: &str, now_ms: i64) {
        self.last_checked_at_ms = Some(now_ms);
        self.failure_count += 1;
        self.last_error = Some(error.to_owned());
        let exponent = self.failure_count.saturating_sub(1).min(8) as u32;
        let backoff_ms = BASE_BACKOFF_MS
            .saturating_mul(2_i64.saturating_pow(exponent))
            .min(MAX_BACKOFF_MS);
        self.backoff_until_ms = Some(now_ms.saturating_add(backoff_ms));
    }
}

#[derive(Debug, Default)]
pub(crate) struct SourceFetchStates {
    states: BTreeMap<String, SourceFetchState>,
}

impl SourceFetchStates {
    pub(crate) async fn load(
        object_store: Option<&ObjectStore>,
        sources: &[&Source],
    ) -> Result<Self, Box<dyn Error>> {
        let mut states = BTreeMap::new();
        for source in sources {
            let state = if let Some(object_store) = object_store {
                load_state(object_store, source).await?
            } else {
                SourceFetchState::new(source)
            };
            states.insert(source.source_id.clone(), state);
        }
        Ok(Self { states })
    }

    pub(crate) fn get(&self, source: &Source) -> Option<&SourceFetchState> {
        self.states.get(&source.source_id)
    }

    pub(crate) fn get_mut(&mut self, source: &Source) -> &mut SourceFetchState {
        self.states
            .entry(source.source_id.clone())
            .or_insert_with(|| SourceFetchState::new(source))
    }

    pub(crate) async fn persist(
        &self,
        object_store: Option<&ObjectStore>,
    ) -> Result<(), Box<dyn Error>> {
        let Some(object_store) = object_store else {
            return Ok(());
        };
        for state in self.states.values() {
            let bytes = serde_json::to_vec_pretty(state)?;
            object_store
                .put_bytes(
                    &state_object_key(&state.source_id),
                    bytes,
                    "application/json",
                )
                .await?;
        }
        Ok(())
    }
}

async fn load_state(
    object_store: &ObjectStore,
    source: &Source,
) -> Result<SourceFetchState, Box<dyn Error>> {
    let key = state_object_key(&source.source_id);
    if !object_store.key_exists(&key).await? {
        return Ok(SourceFetchState::new(source));
    }
    let raw = object_store.get_bytes(&key).await?;
    let mut state = serde_json::from_slice::<SourceFetchState>(&raw)?;
    if state.schema_version != SOURCE_FETCH_STATE_SCHEMA || state.source_id != source.source_id {
        state = SourceFetchState::new(source);
    }
    Ok(state)
}

fn state_object_key(source_id: &str) -> String {
    format!(
        "source-fetch-state/schema={SOURCE_FETCH_STATE_SCHEMA}/source_id={}/state.json",
        path_segment(source_id)
    )
}

fn state_id(source_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_id.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .chars()
        .take(24)
        .collect::<String>();
    format!("source_fetch_state_{suffix}")
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

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{AppliesToAssets, Source};

    #[test]
    fn backs_off_exponentially_after_failures() {
        let source = source();
        let mut state = SourceFetchState::new(&source);

        state.record_failure("HTTP 502", 1_000);
        assert_eq!(state.backoff_until_ms(), Some(61_000));

        state.record_failure("HTTP 502", 2_000);
        assert_eq!(state.backoff_until_ms(), Some(122_000));
    }

    #[test]
    fn records_conditional_get_metadata() {
        let source = source();
        let mut state = SourceFetchState::new(&source);
        let metadata = FetchMetadata {
            http_status: 200,
            etag: Some("\"abc\"".to_owned()),
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned()),
        };

        state.record_success(&metadata, 1_000, 1);

        let headers = state.cache_headers();
        assert_eq!(headers.etag.as_deref(), Some("\"abc\""));
        assert_eq!(
            headers.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
    }

    #[test]
    fn records_not_modified_without_losing_cache_metadata() {
        let source = source();
        let mut state = SourceFetchState::new(&source);
        state.record_success(
            &FetchMetadata {
                http_status: 200,
                etag: Some("\"abc\"".to_owned()),
                last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_owned()),
            },
            1_000,
            1,
        );

        state.record_not_modified(
            &FetchMetadata {
                http_status: 304,
                etag: None,
                last_modified: None,
            },
            2_000,
        );

        assert_eq!(state.last_http_status, Some(304));
        assert_eq!(state.last_checked_at_ms, Some(2_000));
        assert_eq!(state.last_success_at_ms, Some(2_000));
        assert_eq!(state.unchanged_count, 1);
        assert_eq!(state.failure_count, 0);
        assert_eq!(state.backoff_until_ms(), None);
        let headers = state.cache_headers();
        assert_eq!(headers.etag.as_deref(), Some("\"abc\""));
        assert_eq!(
            headers.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
    }

    #[test]
    fn clears_backoff_after_success() {
        let source = source();
        let mut state = SourceFetchState::new(&source);
        state.record_failure("HTTP 503", 1_000);
        assert!(state.is_backing_off(1_500));

        state.record_success(
            &FetchMetadata {
                http_status: 200,
                etag: None,
                last_modified: None,
            },
            2_000,
            0,
        );

        assert_eq!(state.failure_count, 0);
        assert_eq!(state.backoff_until_ms(), None);
        assert!(!state.is_backing_off(2_001));
    }

    #[test]
    fn state_key_sanitizes_source_id() {
        assert_eq!(
            state_object_key("news/binance announcements"),
            "source-fetch-state/schema=source_fetch_state_v1/source_id=news_binance_announcements/state.json"
        );
    }

    fn source() -> Source {
        Source {
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
        }
    }
}

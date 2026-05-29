use super::{BASE_BACKOFF_MS, MAX_BACKOFF_MS, SOURCE_FETCH_STATE_SCHEMA};
use crate::fetch::{CacheHeaders, FetchMetadata};
use crate::registry::Source;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SourceFetchState {
    pub(super) schema_version: String,
    pub(super) source_id: String,
    state_id: String,
    etag: Option<String>,
    last_modified: Option<String>,
    pub(super) last_http_status: Option<u16>,
    pub(super) last_checked_at_ms: Option<i64>,
    last_success_at_ms: Option<i64>,
    pub(super) unchanged_count: usize,
    pub(super) failure_count: usize,
    backoff_until_ms: Option<i64>,
    last_error: Option<String>,
}

impl SourceFetchState {
    pub(crate) fn new(source: &Source) -> Self {
        Self {
            schema_version: SOURCE_FETCH_STATE_SCHEMA.to_owned(),
            source_id: source.source_id.clone(),
            state_id: super::keys::state_id(&source.source_id),
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

    pub(crate) fn last_success_at_ms(&self) -> Option<i64> {
        self.last_success_at_ms
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

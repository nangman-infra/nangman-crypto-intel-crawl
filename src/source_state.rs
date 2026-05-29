use chrono::Utc;

mod collection;
mod keys;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use collection::SourceFetchStates;

const SOURCE_FETCH_STATE_SCHEMA: &str = "source_fetch_state_v1";
const BASE_BACKOFF_MS: i64 = 60_000;
const MAX_BACKOFF_MS: i64 = 30 * 60_000;

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

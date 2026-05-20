#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedItem {
    pub(crate) id: Option<String>,
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) url: String,
    pub(crate) author: Option<String>,
    pub(crate) published_at: Option<String>,
    pub(crate) historical_source_depth: Option<String>,
    pub(crate) backfill_window_start_ms: Option<i64>,
    pub(crate) backfill_window_end_ms: Option<i64>,
    pub(crate) source_time_range_verified: Option<bool>,
}

use super::super::types::{FundingHistoryFetchContext, FundingHistoryPage, has_capacity_for_more};
use crate::item::FeedItem;

pub(super) fn should_fetch_funding_history_page(
    context: &FundingHistoryFetchContext<'_>,
    cursor_ms: i64,
    items: &[FeedItem],
) -> bool {
    cursor_ms < context.backfill_end_ms && has_capacity_for_more(context, items)
}

pub(super) fn funding_history_request_limit(
    context: &FundingHistoryFetchContext<'_>,
    items: &[FeedItem],
) -> usize {
    (context.max_items - items.len()).min(1000)
}

pub(super) fn next_funding_history_cursor(
    page: &FundingHistoryPage,
    request_limit: usize,
    last_funding_time: i64,
    cursor_ms: i64,
) -> Option<i64> {
    if page.records.len() < request_limit {
        return None;
    }
    let next_cursor = last_funding_time.saturating_add(1);
    if next_cursor <= cursor_ms {
        None
    } else {
        Some(next_cursor)
    }
}

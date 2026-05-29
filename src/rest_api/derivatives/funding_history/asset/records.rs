use super::super::super::{BinanceFundingRate, binance_funding_rate_history_item};
use super::super::types::FundingHistoryPage;
use crate::item::FeedItem;

pub(super) fn append_funding_history_records(
    items: &mut Vec<FeedItem>,
    max_items: usize,
    backfill_start_ms: i64,
    backfill_end_ms: i64,
    page: &FundingHistoryPage,
    cursor_ms: i64,
) -> i64 {
    let mut last_funding_time = cursor_ms;
    for record in &page.records {
        last_funding_time = last_funding_time.max(record.funding_time);
        if !record_in_backfill_window(record, backfill_start_ms, backfill_end_ms) {
            continue;
        }
        items.push(binance_funding_rate_history_item(
            record,
            &page.url,
            backfill_start_ms,
            backfill_end_ms,
        ));
        if items.len() >= max_items {
            break;
        }
    }
    last_funding_time
}

fn record_in_backfill_window(
    record: &BinanceFundingRate,
    backfill_start_ms: i64,
    backfill_end_ms: i64,
) -> bool {
    record.funding_time >= backfill_start_ms && record.funding_time <= backfill_end_ms
}

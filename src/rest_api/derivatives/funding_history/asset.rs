use self::cursor::{
    funding_history_request_limit, next_funding_history_cursor, should_fetch_funding_history_page,
};
use self::page::fetch_funding_history_page;
use self::records::append_funding_history_records;
use super::types::{FundingHistoryAssetOutcome, FundingHistoryFetchContext};
use crate::item::FeedItem;
use crate::registry::UniverseAsset;

mod cursor;
mod page;
mod records;

pub(super) async fn append_asset_funding_history(
    context: &FundingHistoryFetchContext<'_>,
    asset: &UniverseAsset,
    items: &mut Vec<FeedItem>,
) -> FundingHistoryAssetOutcome {
    let mut cursor_ms = context.backfill_start_ms;
    while should_fetch_funding_history_page(context, cursor_ms, items) {
        let request_limit = funding_history_request_limit(context, items);
        let page = fetch_funding_history_page(
            context.client,
            context.source,
            &asset.reference_symbol_native,
            cursor_ms,
            context.backfill_end_ms,
            request_limit,
        )
        .await;
        let Ok(page) = page else {
            return FundingHistoryAssetOutcome::RequestFailed;
        };
        if page.records.is_empty() {
            return FundingHistoryAssetOutcome::Complete;
        }
        let last_funding_time = append_funding_history_records(
            items,
            context.max_items,
            context.backfill_start_ms,
            context.backfill_end_ms,
            &page,
            cursor_ms,
        );
        let Some(next_cursor) =
            next_funding_history_cursor(&page, request_limit, last_funding_time, cursor_ms)
        else {
            return FundingHistoryAssetOutcome::Complete;
        };
        cursor_ms = next_cursor;
    }
    FundingHistoryAssetOutcome::Complete
}

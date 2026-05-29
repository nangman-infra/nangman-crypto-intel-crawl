use crate::item::FeedItem;
use crate::registry::Source;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum FundingHistoryAssetOutcome {
    Complete,
    RequestFailed,
}

pub(super) struct FundingHistoryFetchContext<'a> {
    pub(super) client: &'a reqwest::Client,
    pub(super) source: &'a Source,
    pub(super) max_items: usize,
    pub(super) backfill_start_ms: i64,
    pub(super) backfill_end_ms: i64,
}

pub(super) struct FundingHistoryPage {
    pub(super) url: String,
    pub(super) records: Vec<super::super::BinanceFundingRate>,
}

pub(super) fn has_capacity_for_more(
    context: &FundingHistoryFetchContext<'_>,
    items: &[FeedItem],
) -> bool {
    items.len() < context.max_items
}

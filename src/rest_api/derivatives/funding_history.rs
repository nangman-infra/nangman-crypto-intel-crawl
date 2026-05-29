use super::{Error, FeedItem, Source, UniverseAsset, prioritized_derivatives_assets};

mod asset;
mod types;

use asset::append_asset_funding_history;
use types::{FundingHistoryAssetOutcome, FundingHistoryFetchContext};

pub(in crate::rest_api) async fn fetch_binance_usdm_funding_rate_history(
    client: &reqwest::Client,
    source: &Source,
    assets: &[UniverseAsset],
    max_items: usize,
    backfill_start_ms: i64,
    backfill_end_ms: i64,
) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let mut items = Vec::new();
    let mut failed_requests = 0usize;
    let context = FundingHistoryFetchContext {
        client,
        source,
        max_items,
        backfill_start_ms,
        backfill_end_ms,
    };
    for asset in prioritized_derivatives_assets(assets) {
        if append_asset_funding_history(&context, asset, &mut items).await
            == FundingHistoryAssetOutcome::RequestFailed
        {
            failed_requests += 1;
        }
        if items.len() >= max_items {
            break;
        }
    }
    if items.is_empty() && failed_requests > 0 {
        return Err(format!(
            "{} returned no usable funding history records after {} failed asset requests",
            source.source_id, failed_requests
        )
        .into());
    }
    Ok(items)
}

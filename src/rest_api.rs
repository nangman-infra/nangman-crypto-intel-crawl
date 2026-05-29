use crate::fetch::{CacheHeaders, FetchMetadata, SourceFetchResult};
use crate::item::FeedItem;
use crate::registry::{Source, UniverseAsset};
use binance_cms::fetch_binance_cms_announcements;
use derivatives::{
    fetch_binance_usdm_funding_rate_history, fetch_binance_usdm_funding_rates,
    fetch_binance_usdm_open_interest, required_backfill_window,
};
use std::error::Error;

mod binance_cms;
mod derivatives;

pub(crate) struct RestFetchOptions<'a> {
    pub(crate) assets: &'a [UniverseAsset],
    pub(crate) cache_headers: Option<&'a CacheHeaders>,
    pub(crate) max_items: usize,
    pub(crate) backfill_start_ms: Option<i64>,
    pub(crate) backfill_end_ms: Option<i64>,
    pub(crate) selection_time_ms: i64,
}

pub(crate) async fn fetch_feed_items(
    client: &reqwest::Client,
    source: &Source,
    options: RestFetchOptions<'_>,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    match source.adapter.as_deref() {
        Some("binance_cms_announcement_list") => {
            fetch_binance_cms_announcements(
                client,
                source,
                options.cache_headers,
                options.max_items,
            )
            .await
        }
        Some("binance_usdm_funding_rate_history") => {
            let (start_ms, end_ms) = required_backfill_window(
                source,
                options.backfill_start_ms,
                options.backfill_end_ms,
            )?;
            let items = fetch_binance_usdm_funding_rate_history(
                client,
                source,
                options.assets,
                options.max_items,
                start_ms,
                end_ms,
            )
            .await?;
            Ok(fetched_without_cache_metadata(items))
        }
        Some("binance_usdm_funding_rate_latest") => {
            let items = fetch_binance_usdm_funding_rates(
                client,
                source,
                options.assets,
                options.max_items,
                options.selection_time_ms,
            )
            .await?;
            Ok(fetched_without_cache_metadata(items))
        }
        Some("binance_usdm_open_interest") => {
            let items = fetch_binance_usdm_open_interest(
                client,
                source,
                options.assets,
                options.max_items,
                options.selection_time_ms,
            )
            .await?;
            Ok(fetched_without_cache_metadata(items))
        }
        Some(adapter) => {
            Err(format!("{} unknown rest_api adapter {adapter}", source.source_id).into())
        }
        None => Err(format!("{} rest_api adapter is missing", source.source_id).into()),
    }
}

fn fetched_without_cache_metadata(items: Vec<FeedItem>) -> SourceFetchResult {
    SourceFetchResult::Fetched {
        items,
        metadata: FetchMetadata {
            http_status: 200,
            etag: None,
            last_modified: None,
        },
    }
}

#[cfg(test)]
mod tests;

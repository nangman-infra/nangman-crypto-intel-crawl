use crate::fetch::{
    CacheHeaders, FetchMetadata, SourceFetchResult, apply_cache_headers, metadata_from_headers,
};
use crate::item::FeedItem;
use crate::registry::{Source, UniverseAsset};
use serde::Deserialize;
use serde_json::json;
use std::error::Error;
use tokio::time::{Duration, sleep};

const LIVE_DERIVATIVES_SELECTION_ROTATION_MS: i64 = 30 * 60_000;

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

async fn fetch_binance_cms_announcements(
    client: &reqwest::Client,
    source: &Source,
    cache_headers: Option<&CacheHeaders>,
    max_items: usize,
) -> Result<SourceFetchResult, Box<dyn Error>> {
    let response = get_json_response_with_retry(client, &source.source_url, cache_headers).await?;
    let status = response.status();
    let metadata = metadata_from_headers(status, response.headers());
    if status == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(SourceFetchResult::NotModified { metadata });
    }
    if !status.is_success() {
        return Err(format!("{} returned HTTP {}", source.source_id, status.as_u16()).into());
    }
    let payload = response.json::<BinanceCmsResponse>().await?;
    if payload.code != "000000" {
        return Err(format!(
            "{} returned Binance code {}",
            source.source_id, payload.code
        )
        .into());
    }

    let mut items = Vec::new();
    if let Some(data) = payload.data {
        for catalog in data.catalogs {
            for article in &catalog.articles {
                sleep(Duration::from_millis(100)).await;
                let body = fetch_binance_cms_article_body(client, article)
                    .await
                    .unwrap_or_else(|| {
                        binance_cms_article_metadata_body(&source.source_id, &catalog, article)
                    });
                items.push(binance_cms_article_item(article, body));
                if items.len() >= max_items {
                    return Ok(SourceFetchResult::Fetched { items, metadata });
                }
            }
        }
    }
    Ok(SourceFetchResult::Fetched { items, metadata })
}

async fn get_json_response_with_retry(
    client: &reqwest::Client,
    url: &str,
    cache_headers: Option<&CacheHeaders>,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut last_response = None;
    for attempt in 0..=1 {
        let request = client.get(url).header("Accept", "application/json");
        let response = apply_cache_headers(request, cache_headers).send().await?;
        if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Ok(response);
        }
        last_response = Some(response);
        if attempt == 0 {
            sleep(Duration::from_secs(2)).await;
        }
    }
    if let Some(response) = last_response {
        Ok(response)
    } else {
        let request = client.get(url).header("Accept", "application/json");
        apply_cache_headers(request, cache_headers).send().await
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

fn binance_cms_article_item(article: &BinanceCmsArticle, body: String) -> FeedItem {
    let url = format!(
        "https://www.binance.com/en/support/announcement/detail/{}",
        article.code
    );

    FeedItem {
        id: Some(article.code.clone()),
        title: article.title.clone(),
        body,
        url,
        author: Some("Binance".to_owned()),
        published_at: Some(article.release_date.to_string()),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

async fn fetch_binance_cms_article_body(
    client: &reqwest::Client,
    article: &BinanceCmsArticle,
) -> Option<String> {
    let url = format!(
        "https://www.binance.com/bapi/composite/v1/public/cms/article/detail/query?articleCode={}",
        article.code
    );
    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload = response.json::<BinanceCmsDetailResponse>().await.ok()?;
    if payload.code != "000000" {
        return None;
    }
    let detail = payload.data?;
    let body_json = serde_json::from_str::<serde_json::Value>(&detail.body).ok()?;
    let mut parts = Vec::new();
    collect_binance_text(&body_json, &mut parts);
    let body = parts.join(" ");
    if body.trim().is_empty() {
        None
    } else {
        Some(body)
    }
}

fn collect_binance_text(value: &serde_json::Value, parts: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("node").and_then(serde_json::Value::as_str) == Some("text")
                && let Some(text) = map.get("text").and_then(serde_json::Value::as_str)
            {
                parts.push(text.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            for child in map.values() {
                collect_binance_text(child, parts);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                collect_binance_text(child, parts);
            }
        }
        _ => {}
    }
}

fn binance_cms_article_metadata_body(
    source_id: &str,
    catalog: &BinanceCmsCatalog,
    article: &BinanceCmsArticle,
) -> String {
    json!({
        "catalog_id": catalog.catalog_id,
        "catalog_name": catalog.catalog_name,
        "article_id": article.id,
        "article_code": article.code,
        "source_id": source_id
    })
    .to_string()
}

async fn fetch_binance_usdm_funding_rates(
    client: &reqwest::Client,
    source: &Source,
    assets: &[UniverseAsset],
    max_items: usize,
    selection_time_ms: i64,
) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let mut items = Vec::new();
    let mut failed_requests = 0usize;
    for asset in prioritized_live_derivatives_assets(assets, &source.source_id, selection_time_ms)
        .into_iter()
        .take(max_items)
    {
        let url = with_query(
            &source.source_url,
            &[("symbol", &asset.reference_symbol_native), ("limit", "1")],
        );
        let Ok(response) = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
        else {
            failed_requests += 1;
            continue;
        };
        if !response.status().is_success() {
            failed_requests += 1;
            continue;
        }
        let Ok(records) = response.json::<Vec<BinanceFundingRate>>().await else {
            failed_requests += 1;
            continue;
        };
        if let Some(record) = records.into_iter().next() {
            items.push(binance_funding_rate_item(&record, &url));
        }
    }
    if items.is_empty() && failed_requests > 0 {
        return Err(format!(
            "{} returned no usable funding records after {} failed asset requests",
            source.source_id, failed_requests
        )
        .into());
    }
    Ok(items)
}

async fn fetch_binance_usdm_funding_rate_history(
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

struct FundingHistoryFetchContext<'a> {
    client: &'a reqwest::Client,
    source: &'a Source,
    max_items: usize,
    backfill_start_ms: i64,
    backfill_end_ms: i64,
}

#[derive(Debug, PartialEq, Eq)]
enum FundingHistoryAssetOutcome {
    Complete,
    RequestFailed,
}

async fn append_asset_funding_history(
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

fn should_fetch_funding_history_page(
    context: &FundingHistoryFetchContext<'_>,
    cursor_ms: i64,
    items: &[FeedItem],
) -> bool {
    cursor_ms < context.backfill_end_ms && items.len() < context.max_items
}

fn funding_history_request_limit(
    context: &FundingHistoryFetchContext<'_>,
    items: &[FeedItem],
) -> usize {
    (context.max_items - items.len()).min(1000)
}

fn next_funding_history_cursor(
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

struct FundingHistoryPage {
    url: String,
    records: Vec<BinanceFundingRate>,
}

async fn fetch_funding_history_page(
    client: &reqwest::Client,
    source: &Source,
    symbol: &str,
    cursor_ms: i64,
    backfill_end_ms: i64,
    request_limit: usize,
) -> Result<FundingHistoryPage, Box<dyn Error>> {
    let request_limit = request_limit.to_string();
    let url = binance_funding_rate_history_url(
        &source.source_url,
        symbol,
        cursor_ms,
        backfill_end_ms,
        &request_limit,
    );
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(format!("{} returned HTTP {}", source.source_id, response.status()).into());
    }
    Ok(FundingHistoryPage {
        url,
        records: response.json::<Vec<BinanceFundingRate>>().await?,
    })
}

fn append_funding_history_records(
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

fn binance_funding_rate_item(record: &BinanceFundingRate, url: &str) -> FeedItem {
    let body = json!({
        "symbol": record.symbol,
        "funding_rate": record.funding_rate,
        "funding_time_ms": record.funding_time,
        "mark_price": record.mark_price
    })
    .to_string();

    FeedItem {
        id: Some(format!("{}:{}", record.symbol, record.funding_time)),
        title: format!("Binance USD-M funding rate {}", record.symbol),
        body,
        url: url.to_owned(),
        author: Some("Binance Futures".to_owned()),
        published_at: Some(record.funding_time.to_string()),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

fn binance_funding_rate_history_item(
    record: &BinanceFundingRate,
    url: &str,
    backfill_start_ms: i64,
    backfill_end_ms: i64,
) -> FeedItem {
    let source_time_range_verified =
        record.funding_time >= backfill_start_ms && record.funding_time <= backfill_end_ms;
    let body = json!({
        "symbol": record.symbol,
        "funding_rate": record.funding_rate,
        "funding_time_ms": record.funding_time,
        "mark_price": record.mark_price,
        "historical_source_depth": "range_queryable",
        "backfill_window_start_ms": backfill_start_ms,
        "backfill_window_end_ms": backfill_end_ms,
        "source_time_range_verified": source_time_range_verified
    })
    .to_string();

    FeedItem {
        id: Some(format!("{}:{}", record.symbol, record.funding_time)),
        title: format!("Binance USD-M funding rate history {}", record.symbol),
        body,
        url: url.to_owned(),
        author: Some("Binance Futures".to_owned()),
        published_at: Some(record.funding_time.to_string()),
        historical_source_depth: Some("range_queryable".to_owned()),
        backfill_window_start_ms: Some(backfill_start_ms),
        backfill_window_end_ms: Some(backfill_end_ms),
        source_time_range_verified: Some(source_time_range_verified),
    }
}

async fn fetch_binance_usdm_open_interest(
    client: &reqwest::Client,
    source: &Source,
    assets: &[UniverseAsset],
    max_items: usize,
    selection_time_ms: i64,
) -> Result<Vec<FeedItem>, Box<dyn Error>> {
    let mut items = Vec::new();
    let mut failed_requests = 0usize;
    for asset in prioritized_live_derivatives_assets(assets, &source.source_id, selection_time_ms)
        .into_iter()
        .take(max_items)
    {
        let url = with_query(
            &source.source_url,
            &[("symbol", &asset.reference_symbol_native)],
        );
        let Ok(response) = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
        else {
            failed_requests += 1;
            continue;
        };
        if !response.status().is_success() {
            failed_requests += 1;
            continue;
        }
        let Ok(record) = response.json::<BinanceOpenInterest>().await else {
            failed_requests += 1;
            continue;
        };
        items.push(binance_open_interest_item(&record, &url));
    }
    if items.is_empty() && failed_requests > 0 {
        return Err(format!(
            "{} returned no usable open interest records after {} failed asset requests",
            source.source_id, failed_requests
        )
        .into());
    }
    Ok(items)
}

fn binance_open_interest_item(record: &BinanceOpenInterest, url: &str) -> FeedItem {
    let body = json!({
        "symbol": record.symbol,
        "open_interest": record.open_interest,
        "event_time_ms": record.time
    })
    .to_string();

    FeedItem {
        id: Some(format!("{}:{}", record.symbol, record.time)),
        title: format!("Binance USD-M open interest {}", record.symbol),
        body,
        url: url.to_owned(),
        author: Some("Binance Futures".to_owned()),
        published_at: Some(record.time.to_string()),
        historical_source_depth: None,
        backfill_window_start_ms: None,
        backfill_window_end_ms: None,
        source_time_range_verified: None,
    }
}

fn prioritized_derivatives_assets(assets: &[UniverseAsset]) -> Vec<&UniverseAsset> {
    assets
        .iter()
        .filter(|asset| asset.rss_seed_status.as_deref() == Some("asset_specific_verified"))
        .chain(
            assets.iter().filter(|asset| {
                asset.rss_seed_status.as_deref() != Some("asset_specific_verified")
            }),
        )
        .collect()
}

fn prioritized_live_derivatives_assets<'a>(
    assets: &'a [UniverseAsset],
    source_id: &str,
    selection_time_ms: i64,
) -> Vec<&'a UniverseAsset> {
    prioritized_live_derivatives_assets_for_seed(
        assets,
        live_derivatives_selection_seed(source_id, selection_time_ms),
    )
}

fn prioritized_live_derivatives_assets_for_seed(
    assets: &[UniverseAsset],
    selection_seed: usize,
) -> Vec<&UniverseAsset> {
    let verified = rotated_assets(
        assets
            .iter()
            .filter(|asset| asset.rss_seed_status.as_deref() == Some("asset_specific_verified"))
            .collect::<Vec<_>>(),
        selection_seed,
    );
    let global_only = rotated_assets(
        assets
            .iter()
            .filter(|asset| asset.rss_seed_status.as_deref() != Some("asset_specific_verified"))
            .collect::<Vec<_>>(),
        selection_seed,
    );
    interleave_assets(verified, global_only)
}

fn rotated_assets(mut assets: Vec<&UniverseAsset>, selection_seed: usize) -> Vec<&UniverseAsset> {
    if !assets.is_empty() {
        let offset = selection_seed % assets.len();
        assets.rotate_left(offset);
    }
    assets
}

fn interleave_assets<'a>(
    primary: Vec<&'a UniverseAsset>,
    secondary: Vec<&'a UniverseAsset>,
) -> Vec<&'a UniverseAsset> {
    let mut ranked = Vec::with_capacity(primary.len() + secondary.len());
    let max_len = primary.len().max(secondary.len());
    for index in 0..max_len {
        if let Some(asset) = primary.get(index) {
            ranked.push(*asset);
        }
        if let Some(asset) = secondary.get(index) {
            ranked.push(*asset);
        }
    }
    ranked
}

fn live_derivatives_selection_seed(source_id: &str, selection_time_ms: i64) -> usize {
    let time_slot = selection_time_ms.max(0) / LIVE_DERIVATIVES_SELECTION_ROTATION_MS;
    stable_source_offset(source_id).wrapping_add(time_slot as usize)
}

fn stable_source_offset(source_id: &str) -> usize {
    source_id.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as usize)
    })
}

fn with_query(base_url: &str, params: &[(&str, &str)]) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base_url}{separator}{query}")
}

fn binance_funding_rate_history_url(
    base_url: &str,
    symbol: &str,
    start_time_ms: i64,
    end_time_ms: i64,
    limit: &str,
) -> String {
    with_query(
        base_url,
        &[
            ("symbol", symbol),
            ("startTime", &start_time_ms.to_string()),
            ("endTime", &end_time_ms.to_string()),
            ("limit", limit),
        ],
    )
}

fn required_backfill_window(
    source: &Source,
    backfill_start_ms: Option<i64>,
    backfill_end_ms: Option<i64>,
) -> Result<(i64, i64), Box<dyn Error>> {
    let Some(start_ms) = backfill_start_ms else {
        return Err(format!("{} requires --backfill-start-ms", source.source_id).into());
    };
    let Some(end_ms) = backfill_end_ms else {
        return Err(format!("{} requires --backfill-end-ms", source.source_id).into());
    };
    if start_ms >= end_ms {
        return Err(format!(
            "{} requires backfill_start_ms < backfill_end_ms",
            source.source_id
        )
        .into());
    }
    Ok((start_ms, end_ms))
}

#[derive(Debug, Deserialize)]
struct BinanceCmsResponse {
    code: String,
    data: Option<BinanceCmsData>,
}

#[derive(Debug, Deserialize)]
struct BinanceCmsData {
    catalogs: Vec<BinanceCmsCatalog>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceCmsCatalog {
    catalog_id: u64,
    catalog_name: String,
    #[serde(default)]
    articles: Vec<BinanceCmsArticle>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceCmsArticle {
    id: u64,
    code: String,
    title: String,
    release_date: i64,
}

#[derive(Debug, Deserialize)]
struct BinanceCmsDetailResponse {
    code: String,
    data: Option<BinanceCmsDetail>,
}

#[derive(Debug, Deserialize)]
struct BinanceCmsDetail {
    body: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceFundingRate {
    symbol: String,
    funding_rate: String,
    funding_time: i64,
    mark_price: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceOpenInterest {
    symbol: String,
    open_interest: String,
    time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_binance_cms_article_item() {
        let catalog = BinanceCmsCatalog {
            catalog_id: 48,
            catalog_name: "New Cryptocurrency Listing".to_owned(),
            articles: Vec::new(),
        };
        let article = BinanceCmsArticle {
            id: 1,
            code: "abc".to_owned(),
            title: "Binance lists TEST".to_owned(),
            release_date: 1778137169304,
        };

        let body =
            binance_cms_article_metadata_body("exchange_binance_listing_rest", &catalog, &article);
        let item = binance_cms_article_item(&article, body);

        assert_eq!(item.id.as_deref(), Some("abc"));
        assert_eq!(item.published_at.as_deref(), Some("1778137169304"));
        assert!(item.url.ends_with("/abc"));
        assert!(item.body.contains("exchange_binance_listing_rest"));
    }

    #[test]
    fn extracts_binance_rich_text_body() {
        let raw = json!({
            "node": "root",
            "child": [
                {"node": "element", "child": [{"node": "text", "text": "Fellow Binancians,"}]},
                {"node": "text", "text": "Trading starts soon."}
            ]
        });
        let mut parts = Vec::new();

        collect_binance_text(&raw, &mut parts);

        assert_eq!(parts.join(" "), "Fellow Binancians, Trading starts soon.");
    }

    #[test]
    fn appends_query_with_existing_params() {
        assert_eq!(
            with_query("https://example.com/path?a=1", &[("symbol", "BTCUSDT")]),
            "https://example.com/path?a=1&symbol=BTCUSDT"
        );
    }

    #[test]
    fn builds_funding_rate_history_url() {
        assert_eq!(
            binance_funding_rate_history_url(
                "https://fapi.binance.com/fapi/v1/fundingRate",
                "BTCUSDT",
                1764892800000,
                1764979200000,
                "1000",
            ),
            "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&startTime=1764892800000&endTime=1764979200000&limit=1000"
        );
    }

    #[test]
    fn funding_history_item_carries_backfill_metadata() {
        let record = BinanceFundingRate {
            symbol: "BTCUSDT".to_owned(),
            funding_rate: "0.00010000".to_owned(),
            funding_time: 1764892800000,
            mark_price: "90000.0".to_owned(),
        };

        let item = binance_funding_rate_history_item(
            &record,
            "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT",
            1764892800000,
            1764979200000,
        );

        assert_eq!(
            item.historical_source_depth.as_deref(),
            Some("range_queryable")
        );
        assert_eq!(item.backfill_window_start_ms, Some(1764892800000));
        assert_eq!(item.backfill_window_end_ms, Some(1764979200000));
        assert_eq!(item.source_time_range_verified, Some(true));
        assert!(item.body.contains("\"source_time_range_verified\":true"));
    }

    #[test]
    fn derivatives_assets_prioritize_verified_asset_specific_symbols() {
        let assets = vec![
            asset("USDC", "global_news_only"),
            asset("BTC", "asset_specific_verified"),
            asset("ETH", "asset_specific_verified"),
            asset("TST", "global_news_only"),
        ];

        let ranked = prioritized_derivatives_assets(&assets)
            .into_iter()
            .map(|asset| asset.asset.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ranked, vec!["BTC", "ETH", "USDC", "TST"]);
    }

    #[test]
    fn live_derivatives_assets_interleave_verified_and_global_only_symbols() {
        let assets = vec![
            asset("USDC", "asset_specific_verified"),
            asset("BTC", "asset_specific_verified"),
            asset("ETH", "asset_specific_verified"),
            asset("SOL", "asset_specific_verified"),
            asset("TST", "global_news_only"),
            asset("DOGS", "global_news_only"),
            asset("CHIP", "global_news_only"),
        ];

        let ranked = prioritized_live_derivatives_assets_for_seed(&assets, 0)
            .into_iter()
            .map(|asset| asset.asset.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ranked,
            vec!["USDC", "TST", "BTC", "DOGS", "ETH", "CHIP", "SOL"]
        );
    }

    #[test]
    fn live_derivatives_assets_rotate_between_selection_windows() {
        let assets = vec![
            asset("USDC", "asset_specific_verified"),
            asset("BTC", "asset_specific_verified"),
            asset("ETH", "asset_specific_verified"),
            asset("SOL", "asset_specific_verified"),
            asset("TON", "asset_specific_verified"),
            asset("ZEC", "asset_specific_verified"),
            asset("TST", "global_news_only"),
            asset("DOGS", "global_news_only"),
            asset("CHIP", "global_news_only"),
            asset("PEPE", "global_news_only"),
        ];

        let first_window = prioritized_live_derivatives_assets(&assets, "funding", 0)
            .into_iter()
            .take(6)
            .map(|asset| asset.asset.as_str())
            .collect::<Vec<_>>();
        let second_window = prioritized_live_derivatives_assets(
            &assets,
            "funding",
            LIVE_DERIVATIVES_SELECTION_ROTATION_MS,
        )
        .into_iter()
        .take(6)
        .map(|asset| asset.asset.as_str())
        .collect::<Vec<_>>();

        assert_ne!(first_window, second_window);
        assert_eq!(first_window.len(), 6);
        assert_eq!(second_window.len(), 6);
    }

    #[test]
    fn live_derivatives_sources_use_different_asset_offsets() {
        let assets = vec![
            asset("USDC", "asset_specific_verified"),
            asset("BTC", "asset_specific_verified"),
            asset("ETH", "asset_specific_verified"),
            asset("SOL", "asset_specific_verified"),
            asset("TON", "asset_specific_verified"),
            asset("ZEC", "asset_specific_verified"),
            asset("TST", "global_news_only"),
            asset("DOGS", "global_news_only"),
            asset("CHIP", "global_news_only"),
            asset("PEPE", "global_news_only"),
        ];

        let funding_window = prioritized_live_derivatives_assets(
            &assets,
            "derivatives_binance_usdm_funding_rate_rest",
            0,
        )
        .into_iter()
        .take(6)
        .map(|asset| asset.asset.as_str())
        .collect::<Vec<_>>();
        let open_interest_window = prioritized_live_derivatives_assets(
            &assets,
            "derivatives_binance_usdm_open_interest_rest",
            0,
        )
        .into_iter()
        .take(6)
        .map(|asset| asset.asset.as_str())
        .collect::<Vec<_>>();

        assert_ne!(funding_window, open_interest_window);
    }

    fn asset(asset: &str, rss_seed_status: &str) -> UniverseAsset {
        UniverseAsset {
            asset: asset.to_owned(),
            reference_symbol_native: format!("{asset}USDT"),
            rss_seed_status: Some(rss_seed_status.to_owned()),
        }
    }
}

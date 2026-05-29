use super::{
    BinanceFundingRate, BinanceOpenInterest, Error, FeedItem, Source, UniverseAsset,
    binance_funding_rate_item, binance_open_interest_item, prioritized_live_derivatives_assets,
    with_query,
};

pub(in crate::rest_api) async fn fetch_binance_usdm_funding_rates(
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

pub(in crate::rest_api) async fn fetch_binance_usdm_open_interest(
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

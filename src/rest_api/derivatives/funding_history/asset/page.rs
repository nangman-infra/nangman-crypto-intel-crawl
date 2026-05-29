use super::super::super::{BinanceFundingRate, binance_funding_rate_history_url};
use super::super::types::FundingHistoryPage;
use crate::registry::Source;
use std::error::Error;

pub(super) async fn fetch_funding_history_page(
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
